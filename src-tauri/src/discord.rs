//! Discord Rich Presence: "Listening to …" in the user's Discord profile.
//!
//! Talks to the locally running Discord client over its IPC pipe (no network, no token).
//! Discord shows images by fetching a public URL itself, so a cover served by a Navidrome
//! on the home network cannot be shown. Instead the same album is looked up in public
//! catalogues (Cover Art Archive, Apple, Deezer) and that public picture is used; the
//! user's server is never exposed.

use std::collections::HashMap;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Discord allows 5 activity updates per 20 seconds.
const MIN_GAP: Duration = Duration::from_secs(4);
const RECONNECT_EVERY: Duration = Duration::from_secs(15);

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiscordConfig {
    pub enabled: bool,
    pub app_id: String,
    /// Look the cover up in public catalogues.
    pub covers: bool,
    pub hide_paused: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NowPlaying {
    pub title: String,
    pub artist: String,
    pub album: String,
    /// MusicBrainz release id, when the server knows it.
    pub album_mbid: String,
    pub duration: f64,
    pub position: f64,
    pub playing: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscordStatus {
    pub connected: bool,
    pub message: Option<String>,
    /// Public cover URL in use, if one was found.
    pub cover: Option<String>,
}

enum Msg {
    Config(DiscordConfig),
    Update(Option<NowPlaying>),
}

#[derive(Clone)]
pub struct Discord {
    tx: Sender<Msg>,
}

impl Discord {
    pub fn spawn(emit: impl Fn(DiscordStatus) + Send + 'static) -> Discord {
        let (tx, rx) = crossbeam_channel::unbounded();
        let _ = std::thread::Builder::new().name("lumen-discord".into()).spawn(move || Worker::new(rx, emit).run());
        Discord { tx }
    }

    pub fn configure(&self, cfg: DiscordConfig) {
        let _ = self.tx.send(Msg::Config(cfg));
    }

    pub fn update(&self, now: Option<NowPlaying>) {
        let _ = self.tx.send(Msg::Update(now));
    }
}

struct Worker<E: Fn(DiscordStatus)> {
    rx: Receiver<Msg>,
    emit: E,
    cfg: DiscordConfig,
    conn: Option<ipc::Conn>,
    last_attempt: Option<Instant>,
    last_sent: Option<Instant>,
    want: Option<NowPlaying>,
    /// What Discord currently shows, to skip identical updates.
    shown: Option<Value>,
    dirty: bool,
    covers: HashMap<String, Option<String>>,
    cover: Option<String>,
    /// Some Discord versions reject newer activity fields; then we stop sending them.
    simple: bool,
    status: Option<(bool, Option<String>, Option<String>)>,
}

impl<E: Fn(DiscordStatus)> Worker<E> {
    fn new(rx: Receiver<Msg>, emit: E) -> Self {
        Worker {
            rx,
            emit,
            cfg: DiscordConfig::default(),
            conn: None,
            last_attempt: None,
            last_sent: None,
            want: None,
            shown: None,
            dirty: false,
            covers: HashMap::new(),
            cover: None,
            simple: false,
            status: None,
        }
    }

    fn run(mut self) {
        loop {
            match self.rx.recv_timeout(Duration::from_millis(500)) {
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            while let Ok(msg) = self.rx.try_recv() {
                self.handle(msg);
            }
            self.tick();
        }
    }

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Config(cfg) => {
                if cfg.app_id != self.cfg.app_id || !cfg.enabled {
                    self.disconnect(None);
                    self.last_attempt = None;
                }
                if cfg.covers != self.cfg.covers {
                    self.covers.clear();
                }
                self.cfg = cfg;
                self.dirty = true;
            }
            Msg::Update(now) => {
                self.want = now;
                self.dirty = true;
            }
        }
    }

    fn report(&mut self, connected: bool, message: Option<String>) {
        let next = (connected, message.clone(), self.cover.clone());
        if self.status.as_ref() != Some(&next) {
            (self.emit)(DiscordStatus { connected, message, cover: self.cover.clone() });
            self.status = Some(next);
        }
    }

    fn disconnect(&mut self, message: Option<String>) {
        self.conn = None;
        self.shown = None;
        let connected = false;
        self.report(connected, message);
    }

    fn tick(&mut self) {
        let id = self.cfg.app_id.trim().to_string();
        if !self.cfg.enabled || id.is_empty() {
            if self.conn.is_some() {
                self.disconnect(None);
            }
            return;
        }

        if self.conn.is_none() {
            if self.last_attempt.is_some_and(|t| t.elapsed() < RECONNECT_EVERY) {
                return;
            }
            self.last_attempt = Some(Instant::now());
            match ipc::Conn::open(&id) {
                Ok(conn) => {
                    self.conn = Some(conn);
                    self.shown = None;
                    self.dirty = true;
                    self.report(true, None);
                }
                Err(e) => {
                    self.report(false, Some(e));
                    return;
                }
            }
        }

        if !self.dirty || self.last_sent.is_some_and(|t| t.elapsed() < MIN_GAP) {
            return;
        }
        self.dirty = false;

        let activity = match self.want.clone() {
            Some(now) if now.playing || !self.cfg.hide_paused => {
                self.cover = self.lookup_cover(&now);
                Some(self.activity(&now))
            }
            _ => {
                self.cover = None;
                None
            }
        };
        if activity == self.shown {
            return;
        }
        self.last_sent = Some(Instant::now());
        let result = self.conn.as_mut().unwrap().set_activity(activity.clone());
        match result {
            Ok(()) => {
                self.shown = activity;
                self.report(true, None);
            }
            Err(ipc::Error::Rejected(message)) if !self.simple => {
                // Retry once without the optional fields an older client may not know.
                self.simple = true;
                self.dirty = true;
                self.last_sent = None;
                self.report(true, Some(message));
            }
            Err(ipc::Error::Rejected(message)) => self.report(true, Some(message)),
            Err(ipc::Error::Io(e)) => {
                // A client that dislikes a field may simply hang up; drop the optional ones.
                if self.shown.is_none() {
                    self.simple = true;
                }
                self.disconnect(Some(format!("связь с Discord потеряна: {e}")));
            }
        }
    }

    fn activity(&self, now: &NowPlaying) -> Value {
        // Discord requires 2..=128 characters in these fields.
        let field = |s: &str, fallback: &str| {
            let s = if s.trim().is_empty() { fallback } else { s.trim() };
            let mut s: String = s.chars().take(128).collect();
            while s.chars().count() < 2 {
                s.push(' ');
            }
            s
        };
        let mut a = json!({
            "type": 2,
            "details": field(&now.title, "Без названия"),
            "state": field(&if now.playing { now.artist.clone() } else { format!("{} · пауза", now.artist) }, "Неизвестный исполнитель"),
        });
        if now.playing && now.duration > 0.0 {
            let now_ms = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
            // Round to the second so small clock jitter does not count as a change.
            let start = (now_ms - (now.position * 1000.0) as i64) / 1000 * 1000;
            a["timestamps"] = json!({ "start": start, "end": start + (now.duration * 1000.0) as i64 });
        }
        let mut assets = json!({ "large_text": field(&now.album, "Lumen") });
        if let Some(url) = &self.cover {
            assets["large_image"] = json!(url);
        }
        a["assets"] = assets;
        if !self.simple {
            // Show the artist in the member list instead of the application's name.
            a["status_display_type"] = json!(1);
        }
        a
    }

    fn lookup_cover(&mut self, now: &NowPlaying) -> Option<String> {
        if !self.cfg.covers || now.album.trim().is_empty() {
            return None;
        }
        let key = format!("{}\u{0}{}\u{0}{}", now.artist.to_lowercase(), now.album.to_lowercase(), now.album_mbid);
        if let Some(found) = self.covers.get(&key) {
            return found.clone();
        }
        let found = covers::find(&now.artist, &now.album, &now.album_mbid);
        self.covers.insert(key, found.clone());
        found
    }
}

/// Finding a public picture of an album.
pub mod covers {
    use std::time::Duration;

    use serde_json::Value;
    use ureq::ResponseExt;

    fn agent() -> ureq::Agent {
        ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(6)))
            .user_agent("Lumen/0.1 (desktop music player)")
            .build()
            .into()
    }

    fn json(req: ureq::RequestBuilder<ureq::typestate::WithoutBody>) -> Option<Value> {
        let text = req.call().ok()?.into_body().read_to_string().ok()?;
        serde_json::from_str(&text).ok()
    }

    /// Lowercase, letters and digits only, and without "(Deluxe Edition)"-style suffixes.
    pub fn norm(s: &str) -> String {
        let base = s.split(['(', '[']).next().unwrap_or(s);
        base.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
    }

    fn matches(want_artist: &str, want_album: &str, artist: &str, album: &str) -> bool {
        let (wa, wb, a, b) = (norm(want_artist), norm(want_album), norm(artist), norm(album));
        let album_ok = !wb.is_empty() && (wb == b || (wb.len() >= 4 && (b.starts_with(&wb) || wb.starts_with(&b))));
        let artist_ok = wa.is_empty() || a.contains(&wa) || wa.contains(&a);
        album_ok && artist_ok
    }

    /// Cover Art Archive: exact, when the server knows the MusicBrainz release.
    fn cover_art_archive(mbid: &str) -> Option<String> {
        if mbid.len() != 36 || !mbid.chars().all(|c| c.is_ascii_hexdigit() || c == '-') {
            return None;
        }
        let url = format!("https://coverartarchive.org/release/{mbid}/front-500");
        let resp = agent().head(&url).call().ok()?;
        // Follow the redirect ourselves, so Discord gets a direct image link.
        Some(resp.get_uri().to_string())
    }

    fn itunes(artist: &str, album: &str) -> Option<String> {
        let body = json(
            agent()
                .get("https://itunes.apple.com/search")
                .query("term", format!("{artist} {album}"))
                .query("entity", "album")
                .query("limit", "10"),
        )?;
        body["results"].as_array()?.iter().find_map(|r| {
            let ok = matches(artist, album, r["artistName"].as_str()?, r["collectionName"].as_str()?);
            ok.then(|| r["artworkUrl100"].as_str().map(|u| u.replace("100x100bb", "600x600bb")))?
        })
    }

    fn deezer(artist: &str, album: &str) -> Option<String> {
        let body = json(
            agent()
                .get("https://api.deezer.com/search/album")
                .query("q", format!("{artist} {album}"))
                .query("limit", "10"),
        )?;
        body["data"].as_array()?.iter().find_map(|r| {
            let ok = matches(artist, album, r["artist"]["name"].as_str()?, r["title"].as_str()?);
            ok.then(|| r["cover_xl"].as_str().map(str::to_string))?
        })
    }

    /// A public https URL showing this album's cover, if any catalogue has it.
    pub fn find(artist: &str, album: &str, mbid: &str) -> Option<String> {
        cover_art_archive(mbid).or_else(|| itunes(artist, album)).or_else(|| deezer(artist, album))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Hits the real catalogues: run with `cargo test -- --ignored live_lookup`.
        #[test]
        #[ignore]
        fn live_lookup() {
            let mbid = find("Daft Punk", "Discovery", "098060d4-ba3c-469d-88b0-08642261c7ab");
            println!("with mbid: {mbid:?}");
            println!("itunes: {:?}", itunes("Daft Punk", "Discovery"));
            println!("deezer: {:?}", deezer("Daft Punk", "Discovery"));
            assert!(mbid.is_some_and(|u| u.contains("archive.org")));
            assert!(itunes("Daft Punk", "Discovery").is_some() && deezer("Daft Punk", "Discovery").is_some());
        }

        #[test]
        fn album_matching_ignores_case_punctuation_and_editions() {
            assert!(matches("Evanescence", "Fallen", "Evanescence", "Fallen (Deluxe Edition)"));
            assert!(matches("Земфира", "Вендетта", "ЗЕМФИРА", "Вендетта"));
            assert!(matches("AC/DC", "Back in Black", "AC DC", "Back In Black [Remastered]"));
            assert!(!matches("Evanescence", "Fallen", "Evanescence", "The Open Door"));
            assert!(!matches("Evanescence", "Fallen", "Sevendust", "Fallen"));
        }
    }
}

/// Discord's local IPC: length-prefixed JSON frames over a named pipe.
mod ipc {
    use std::fs::File;
    use std::io::{Read, Write};

    use serde_json::{json, Value};

    pub enum Error {
        Io(String),
        /// Discord answered, but refused the request.
        Rejected(String),
    }

    pub struct Conn {
        pipe: File,
        nonce: u64,
    }

    impl Conn {
        #[cfg(windows)]
        pub fn open(app_id: &str) -> Result<Conn, String> {
            if !app_id.chars().all(|c| c.is_ascii_digit()) {
                return Err("ID приложения должен состоять из цифр".into());
            }
            for i in 0..10 {
                let path = format!(r"\\.\pipe\discord-ipc-{i}");
                let Ok(pipe) = std::fs::OpenOptions::new().read(true).write(true).open(&path) else { continue };
                let mut conn = Conn { pipe, nonce: 0 };
                conn.send(0, &json!({ "v": 1, "client_id": app_id })).map_err(|e| e.to_string())?;
                let (op, reply) = conn.recv().map_err(|e| e.to_string())?;
                if op == 2 || reply["evt"] != "READY" {
                    let code = reply["code"].as_i64().unwrap_or(0);
                    return Err(if code == 4000 {
                        "Discord не знает такого ID приложения".into()
                    } else {
                        format!("Discord отказал: {}", reply["message"].as_str().unwrap_or("неизвестная ошибка"))
                    });
                }
                return Ok(conn);
            }
            Err("Discord не запущен".into())
        }

        #[cfg(not(windows))]
        pub fn open(_app_id: &str) -> Result<Conn, String> {
            Err("Discord поддерживается только в Windows".into())
        }

        fn send(&mut self, op: u32, payload: &Value) -> std::io::Result<()> {
            let body = serde_json::to_vec(payload)?;
            let mut frame = Vec::with_capacity(8 + body.len());
            frame.extend_from_slice(&op.to_le_bytes());
            frame.extend_from_slice(&(body.len() as u32).to_le_bytes());
            frame.extend_from_slice(&body);
            self.pipe.write_all(&frame)?;
            self.pipe.flush()
        }

        fn recv(&mut self) -> std::io::Result<(u32, Value)> {
            let mut head = [0u8; 8];
            self.pipe.read_exact(&mut head)?;
            let op = u32::from_le_bytes(head[..4].try_into().unwrap());
            let len = u32::from_le_bytes(head[4..].try_into().unwrap()) as usize;
            if len > 1 << 20 {
                return Err(std::io::Error::other("oversized frame"));
            }
            let mut body = vec![0u8; len];
            self.pipe.read_exact(&mut body)?;
            Ok((op, serde_json::from_slice(&body).unwrap_or(Value::Null)))
        }

        /// Show `activity`, or clear the status with `None`.
        pub fn set_activity(&mut self, activity: Option<Value>) -> Result<(), Error> {
            self.nonce += 1;
            let request = json!({
                "cmd": "SET_ACTIVITY",
                "args": { "pid": std::process::id(), "activity": activity },
                "nonce": self.nonce.to_string(),
            });
            self.send(1, &request).map_err(|e| Error::Io(e.to_string()))?;
            // Read until our own answer; anything else Discord sends is ignored.
            loop {
                let (op, reply) = self.recv().map_err(|e| Error::Io(e.to_string()))?;
                if op == 2 {
                    return Err(Error::Io(reply["message"].as_str().unwrap_or("closed").to_string()));
                }
                if op == 3 {
                    // Ping: answer with pong.
                    self.send(4, &reply).map_err(|e| Error::Io(e.to_string()))?;
                    continue;
                }
                if reply["nonce"] != self.nonce.to_string() {
                    continue;
                }
                if reply["evt"] == "ERROR" {
                    return Err(Error::Rejected(reply["data"]["message"].as_str().unwrap_or("ошибка").to_string()));
                }
                return Ok(());
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        /// Talks to the Discord client on this machine: run with `cargo test -- --ignored live_handshake`.
        #[test]
        #[ignore]
        fn live_handshake() {
            let id = std::env::var("LUMEN_DISCORD_ID").unwrap_or_else(|_| "100000000000000001".into());
            match Conn::open(&id) {
                Ok(_) => println!("handshake accepted"),
                Err(e) => println!("handshake refused: {e}"),
            }
        }

        /// The frame format, checked against a pipe stand-in (a loopback socket pair would
        /// need Discord itself; the encoding is what can go wrong on our side).
        #[test]
        fn frames_are_little_endian_length_prefixed() {
            let payload = json!({ "v": 1, "client_id": "123" });
            let body = serde_json::to_vec(&payload).unwrap();
            let mut expected = 0u32.to_le_bytes().to_vec();
            expected.extend_from_slice(&(body.len() as u32).to_le_bytes());
            expected.extend_from_slice(&body);

            let path = std::env::temp_dir().join(format!("lumen-ipc-frame-{}", std::process::id()));
            let file = std::fs::OpenOptions::new().create(true).truncate(true).read(true).write(true).open(&path).unwrap();
            let mut conn = Conn { pipe: file, nonce: 0 };
            conn.send(0, &payload).unwrap();
            drop(conn);
            assert_eq!(std::fs::read(&path).unwrap(), expected);

            let mut conn = Conn { pipe: std::fs::File::open(&path).unwrap(), nonce: 0 };
            let (op, back) = conn.recv().unwrap();
            assert_eq!((op, back), (0, payload));
            let _ = std::fs::remove_file(&path);
        }
    }
}
