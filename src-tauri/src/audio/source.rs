//! Media sources: local files and progressively downloaded HTTP streams.
//!
//! An HTTP stream is written to a file in the cache directory by a background thread while
//! the decoder reads behind it. Memory use therefore stays flat regardless of file size, and
//! a finished download doubles as a cache entry for the next play.
//!
//! Each track is downloaded at most once at a time: the player and the background prefetcher
//! both go through `fetch`, and whoever comes second joins the download already running.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};

use symphonia::core::io::MediaSource;

use super::TrackReq;

/// How long a read may wait for the network before giving up.
const READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Download progress shared between the pump thread, the reader and the engine.
pub struct Progress {
    pub downloaded: AtomicU64,
    /// Total length in bytes, 0 when the server did not say.
    pub total: AtomicU64,
    pub read_pos: AtomicU64,
    pub done: AtomicBool,
    pub cancel: AtomicBool,
    /// Open players reading this download.
    pub readers: AtomicUsize,
    /// Set by the prefetcher: finish the download even when nobody is listening.
    pub keep: AtomicBool,
    failed: Mutex<Option<String>>,
    wake: (Mutex<()>, Condvar),
}

impl Progress {
    fn new(total: u64) -> Progress {
        Progress {
            downloaded: AtomicU64::new(0),
            total: AtomicU64::new(total),
            read_pos: AtomicU64::new(0),
            done: AtomicBool::new(false),
            cancel: AtomicBool::new(false),
            readers: AtomicUsize::new(0),
            keep: AtomicBool::new(false),
            failed: Mutex::new(None),
            wake: (Mutex::new(()), Condvar::new()),
        }
    }

    pub fn notify(&self) {
        let _guard = self.wake.0.lock().unwrap();
        self.wake.1.notify_all();
    }

    /// Stop the download unless a player is still reading it.
    pub fn release_keep(&self) {
        self.keep.store(false, Ordering::Relaxed);
        if self.readers.load(Ordering::Acquire) == 0 {
            self.cancel.store(true, Ordering::Relaxed);
        }
        self.notify();
    }

    pub fn wait(&self, timeout: Duration) {
        let guard = self.wake.0.lock().unwrap();
        let _ = self.wake.1.wait_timeout(guard, timeout);
    }

    pub fn error(&self) -> Option<String> {
        self.failed.lock().unwrap().clone()
    }

    /// Fraction downloaded, 0..1.
    pub fn fraction(&self) -> f64 {
        if self.done.load(Ordering::Relaxed) {
            return 1.0;
        }
        let total = self.total.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            (self.downloaded.load(Ordering::Relaxed) as f64 / total as f64).min(1.0)
        }
    }

    /// Bytes downloaded but not yet consumed by the decoder.
    pub fn ahead(&self) -> u64 {
        self.downloaded.load(Ordering::Relaxed).saturating_sub(self.read_pos.load(Ordering::Relaxed))
    }
}

/// Reader half of a progressive download.
pub struct StreamSource {
    file: File,
    pos: u64,
    file_pos: u64,
    progress: Arc<Progress>,
}

impl Read for StreamSource {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let p = &self.progress;
        let start = Instant::now();
        loop {
            let avail = p.downloaded.load(Ordering::Acquire);
            if self.pos < avail {
                let n = buf.len().min((avail - self.pos) as usize);
                if self.file_pos != self.pos {
                    self.file.seek(SeekFrom::Start(self.pos))?;
                }
                let got = self.file.read(&mut buf[..n])?;
                self.pos += got as u64;
                self.file_pos = self.pos;
                p.read_pos.store(self.pos, Ordering::Relaxed);
                return Ok(got);
            }
            if p.done.load(Ordering::Acquire) {
                // Re-check: the pump may have finished between the two loads.
                if self.pos < p.downloaded.load(Ordering::Acquire) {
                    continue;
                }
                return Ok(0);
            }
            if let Some(e) = p.error() {
                return Err(io::Error::other(e));
            }
            if p.cancel.load(Ordering::Relaxed) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
            if start.elapsed() > READ_TIMEOUT {
                return Err(io::Error::new(io::ErrorKind::TimedOut, "stream stalled"));
            }
            p.wait(Duration::from_millis(100));
        }
    }
}

impl Seek for StreamSource {
    fn seek(&mut self, to: SeekFrom) -> io::Result<u64> {
        let p = &self.progress;
        let new = match to {
            SeekFrom::Start(n) => n as i128,
            SeekFrom::Current(d) => self.pos as i128 + d as i128,
            SeekFrom::End(d) => {
                let mut total = p.total.load(Ordering::Relaxed);
                let start = Instant::now();
                // Unknown length: the end is only known once the download completes.
                while total == 0 {
                    if p.done.load(Ordering::Acquire) {
                        total = p.downloaded.load(Ordering::Acquire);
                        break;
                    }
                    if p.error().is_some() || p.cancel.load(Ordering::Relaxed) || start.elapsed() > READ_TIMEOUT {
                        return Err(io::Error::other("stream length unknown"));
                    }
                    p.wait(Duration::from_millis(100));
                    total = p.total.load(Ordering::Relaxed);
                }
                total as i128 + d as i128
            }
        };
        if new < 0 {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek before start"));
        }
        self.pos = new as u64;
        p.read_pos.store(self.pos, Ordering::Relaxed);
        Ok(self.pos)
    }
}

impl MediaSource for StreamSource {
    fn is_seekable(&self) -> bool {
        true
    }

    fn byte_len(&self) -> Option<u64> {
        match self.progress.total.load(Ordering::Relaxed) {
            0 => None,
            n => Some(n),
        }
    }
}

impl Drop for StreamSource {
    fn drop(&mut self) {
        // Last listener gone: stop an unfinished download, unless it is being cached ahead.
        let p = &self.progress;
        if p.readers.fetch_sub(1, Ordering::AcqRel) == 1 && !p.keep.load(Ordering::Relaxed) {
            p.cancel.store(true, Ordering::Relaxed);
        }
        p.notify();
    }
}

pub struct Opened {
    pub source: Box<dyn MediaSource>,
    /// Present for network streams that are still (or were just) downloading.
    pub progress: Option<Arc<Progress>>,
}

fn sanitize(key: &str) -> String {
    key.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' }).collect()
}

fn cache_file(cache_dir: &Path, cache_key: &str, suffix: &str) -> Option<PathBuf> {
    if cache_key.is_empty() {
        return None;
    }
    let ext = if suffix.is_empty() { "bin".to_string() } else { sanitize(suffix) };
    Some(cache_dir.join(format!("{}.{}", sanitize(cache_key), ext)))
}

fn cached_path(cache_dir: &Path, req: &TrackReq) -> Option<PathBuf> {
    cache_file(cache_dir, &req.cache_key, &req.suffix)
}

/// Whether a finished copy of the track is in the cache.
pub fn is_cached(cache_dir: &Path, cache_key: &str, suffix: &str) -> bool {
    cache_file(cache_dir, cache_key, suffix).is_some_and(|p| p.is_file())
}

/// Mark a cache entry as recently used so the size limit evicts something else first.
pub fn touch(path: &Path) {
    if let Ok(file) = fs::OpenOptions::new().write(true).open(path) {
        let _ = file.set_modified(std::time::SystemTime::now());
    }
}

static NONCE: AtomicU64 = AtomicU64::new(0);

struct Inflight {
    part: PathBuf,
    progress: Arc<Progress>,
}

/// Downloads currently running, by cache key.
fn inflight() -> &'static Mutex<HashMap<String, Inflight>> {
    static MAP: OnceLock<Mutex<HashMap<String, Inflight>>> = OnceLock::new();
    MAP.get_or_init(Default::default)
}

/// A reader file (for players) and the progress of a running download.
pub type Joined = (Option<File>, Arc<Progress>);

pub enum Fetch {
    /// A complete copy is on disk.
    Cached(PathBuf),
    Downloading(Joined),
}

/// Join a download already running for `key`. The caller holds the map lock, which also
/// guarantees the part file has not been renamed away yet.
fn join(map: &HashMap<String, Inflight>, key: &str, reader: bool, keep: bool) -> Option<Joined> {
    let entry = map.get(key)?;
    let p = &entry.progress;
    if p.cancel.load(Ordering::Relaxed) || p.error().is_some() {
        return None;
    }
    let file = if reader { Some(File::open(&entry.part).ok()?) } else { None };
    if reader {
        p.readers.fetch_add(1, Ordering::AcqRel);
    }
    if keep {
        p.keep.store(true, Ordering::Relaxed);
    }
    Some((file, p.clone()))
}

/// Get a track into the cache: an existing copy, a download already running, or a new one.
/// `reader` asks for a file handle to stream from; `keep` asks for the download to finish
/// even when no player is reading it.
pub fn fetch(req: &TrackReq, cache_dir: &Path, cache_limit: u64, reader: bool, keep: bool) -> Result<Fetch, String> {
    let final_path = cached_path(cache_dir, req);
    let key = req.cache_key.clone();
    if let Some(path) = final_path.as_ref().filter(|p| p.is_file()) {
        return Ok(Fetch::Cached(path.clone()));
    }
    if !key.is_empty() {
        let map = inflight().lock().unwrap();
        if let Some(joined) = join(&map, &key, reader, keep) {
            return Ok(Fetch::Downloading(joined));
        }
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(20)))
        .user_agent("SoSamb0-Music")
        .build()
        .into();
    let resp = agent.get(&req.url).call().map_err(|e| format!("request failed: {e}"))?;

    let mut body = resp.into_body();
    // Subsonic servers report errors as a 200 response carrying XML or JSON.
    let mime = body.mime_type().unwrap_or("").to_ascii_lowercase();
    if mime.contains("json") || mime.contains("xml") {
        let text = body.read_to_string().unwrap_or_default();
        let short: String = text.chars().take(300).collect();
        return Err(format!("server refused the stream: {short}"));
    }
    let total = body.content_length().unwrap_or(0);

    fs::create_dir_all(cache_dir).map_err(|e| format!("cache dir: {e}"))?;
    let part = cache_dir.join(format!(
        "{}.{}.part",
        sanitize(if key.is_empty() { "stream" } else { &key }),
        NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut writer = File::create(&part).map_err(|e| format!("cache file: {e}"))?;
    let reader_file = if reader { Some(File::open(&part).map_err(|e| format!("cache file: {e}"))?) } else { None };

    let progress = Arc::new(Progress::new(total));
    progress.readers.store(reader as usize, Ordering::Relaxed);
    progress.keep.store(keep, Ordering::Relaxed);

    if !key.is_empty() {
        let mut map = inflight().lock().unwrap();
        // Someone else may have started or finished the same track while we connected.
        let raced = if final_path.as_ref().is_some_and(|p| p.is_file()) {
            Some(Fetch::Cached(final_path.clone().unwrap()))
        } else {
            join(&map, &key, reader, keep).map(Fetch::Downloading)
        };
        if let Some(found) = raced {
            drop(map);
            drop((writer, reader_file));
            let _ = fs::remove_file(&part);
            return Ok(found);
        }
        map.insert(key.clone(), Inflight { part: part.clone(), progress: progress.clone() });
    }

    let pump = progress.clone();
    let cache_dir = cache_dir.to_path_buf();
    let spawned = std::thread::Builder::new().name("lumen-download".into()).spawn(move || {
        let mut input = body.into_reader();
        let mut buf = vec![0u8; 64 * 1024];
        let result: Result<(), String> = loop {
            if pump.cancel.load(Ordering::Relaxed) {
                break Err("cancelled".into());
            }
            match input.read(&mut buf) {
                Ok(0) => break Ok(()),
                Ok(n) => {
                    if let Err(e) = writer.write_all(&buf[..n]) {
                        break Err(format!("cache write: {e}"));
                    }
                    pump.downloaded.fetch_add(n as u64, Ordering::AcqRel);
                    pump.notify();
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => break Err(format!("download: {e}")),
            }
        };
        let _ = writer.flush();
        drop(writer);

        let got = pump.downloaded.load(Ordering::Acquire);
        let complete = result.is_ok() && (total == 0 || got == total);
        let mut stored = false;
        {
            // Leave the registry and move the file in one step, so a newcomer finds either
            // the running download or the finished cache entry, never neither.
            let mut map = inflight().lock().unwrap();
            if map.get(&key).is_some_and(|e| Arc::ptr_eq(&e.progress, &pump)) {
                map.remove(&key);
            }
            if complete {
                pump.done.store(true, Ordering::Release);
                stored = matches!(&final_path, Some(dest) if fs::rename(&part, dest).is_ok());
            } else if !pump.cancel.load(Ordering::Relaxed) {
                let msg = result.err().unwrap_or_else(|| "connection closed early".into());
                *pump.failed.lock().unwrap() = Some(msg);
            }
            if !stored {
                let _ = fs::remove_file(&part);
            }
        }
        if stored {
            trim_cache(&cache_dir, cache_limit);
        }
        pump.notify();
    });
    if let Err(e) = spawned {
        inflight().lock().unwrap().remove(&req.cache_key);
        return Err(format!("thread: {e}"));
    }
    Ok(Fetch::Downloading((reader_file, progress)))
}

/// Open the media for a track: a cache hit, a local file, or a progressive download.
pub fn open(req: &TrackReq, cache_dir: &Path, cache_limit: u64) -> Result<Opened, String> {
    let is_http = req.url.starts_with("http://") || req.url.starts_with("https://");
    if !is_http {
        let file = File::open(&req.url).map_err(|e| format!("{}: {e}", req.url))?;
        return Ok(Opened { source: Box::new(file), progress: None });
    }
    // A cache entry can be evicted between the check and the open; then just fetch again.
    for _ in 0..2 {
        match fetch(req, cache_dir, cache_limit, true, false)? {
            Fetch::Cached(path) => {
                if let Ok(file) = File::open(&path) {
                    touch(&path);
                    return Ok(Opened { source: Box::new(file), progress: None });
                }
            }
            Fetch::Downloading((Some(file), progress)) => {
                return Ok(Opened {
                    source: Box::new(StreamSource { file, pos: 0, file_pos: 0, progress: progress.clone() }),
                    progress: Some(progress),
                });
            }
            Fetch::Downloading((None, _)) => return Err("download has no reader".into()),
        }
    }
    Err("cache entry disappeared while opening".into())
}

/// Delete least recently used cache entries until the cache fits `limit` bytes.
pub fn trim_cache(cache_dir: &Path, limit: u64) {
    let Ok(entries) = fs::read_dir(cache_dir) else { return };
    let mut files: Vec<(PathBuf, u64, std::time::SystemTime)> = entries
        .flatten()
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let path = e.path();
            if !meta.is_file() || path.extension().is_some_and(|x| x == "part") {
                return None;
            }
            Some((path, meta.len(), meta.modified().ok()?))
        })
        .collect();
    let mut size: u64 = files.iter().map(|f| f.1).sum();
    files.sort_by_key(|f| f.2);
    for (path, len, _) in files {
        if size <= limit {
            break;
        }
        if fs::remove_file(&path).is_ok() {
            size -= len;
        }
    }
}

/// Remove leftovers from downloads interrupted by a crash or forced exit.
pub fn remove_stale_parts(cache_dir: &Path) {
    let Ok(entries) = fs::read_dir(cache_dir) else { return };
    for e in entries.flatten() {
        if e.path().extension().is_some_and(|x| x == "part") {
            let _ = fs::remove_file(e.path());
        }
    }
}

/// Total size in bytes and number of finished cache entries.
pub fn cache_usage(cache_dir: &Path) -> (u64, u64) {
    let Ok(entries) = fs::read_dir(cache_dir) else { return (0, 0) };
    entries
        .flatten()
        .filter_map(|e| e.metadata().ok().filter(|m| m.is_file()).map(|m| m.len()))
        .fold((0, 0), |(size, count), len| (size + len, count + 1))
}

pub fn clear_cache(cache_dir: &Path) {
    let Ok(entries) = fs::read_dir(cache_dir) else { return };
    for e in entries.flatten() {
        // Files still being played stay open; Windows deletes them once released.
        let _ = fs::remove_file(e.path());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;

    /// A tiny HTTP server that sends `body` slowly and counts the requests it gets.
    fn serve(body: Vec<u8>) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/stream", listener.local_addr().unwrap());
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = hits.clone();
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                counter.fetch_add(1, Ordering::SeqCst);
                let body = body.clone();
                std::thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    while reader.read_line(&mut line).is_ok_and(|n| n > 2) {
                        line.clear();
                    }
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: audio/flac\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len()
                    );
                    let _ = stream.write_all(head.as_bytes());
                    for chunk in body.chunks(16 * 1024) {
                        if stream.write_all(chunk).is_err() {
                            return;
                        }
                        std::thread::sleep(Duration::from_millis(15));
                    }
                });
            }
        });
        (url, hits)
    }

    fn req(url: &str, key: &str) -> TrackReq {
        TrackReq {
            token: 1,
            url: url.into(),
            cache_key: key.into(),
            suffix: "flac".into(),
            duration: 0.0,
            album_id: String::new(),
            replay_gain: Default::default(),
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("lumen-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    fn wait_done(p: &Progress) {
        let start = Instant::now();
        while !p.done.load(Ordering::Acquire) && p.error().is_none() && !p.cancel.load(Ordering::Relaxed) {
            assert!(start.elapsed() < Duration::from_secs(20), "download did not finish");
            p.wait(Duration::from_millis(50));
        }
    }

    #[test]
    fn player_joins_a_prefetch_instead_of_downloading_twice() {
        let body: Vec<u8> = (0..400_000u32).map(|i| (i * 7 % 251) as u8).collect();
        let (url, hits) = serve(body.clone());
        let dir = temp_dir("join");
        let track = req(&url, "song-1");

        let Fetch::Downloading((None, prefetch)) = fetch(&track, &dir, u64::MAX, false, true).unwrap() else {
            panic!("expected a new background download");
        };
        let mut opened = open(&track, &dir, u64::MAX).unwrap();
        let mut got = Vec::new();
        opened.source.read_to_end(&mut got).unwrap();
        assert!(got == body, "player read different bytes");
        wait_done(&prefetch);
        drop(opened);

        assert_eq!(hits.load(Ordering::SeqCst), 1, "the track must be requested once");
        assert!(is_cached(&dir, "song-1", "flac"));
        assert_eq!(fs::read(dir.join("song-1.flac")).unwrap(), body);

        // Next time it comes straight from disk, without the network.
        let again = open(&track, &dir, u64::MAX).unwrap();
        assert!(again.progress.is_none());
        assert_eq!(hits.load(Ordering::SeqCst), 1);
        drop(again);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn skipping_cancels_an_unwanted_download_but_not_a_prefetched_one() {
        let body = vec![1u8; 600_000];
        let (url, _) = serve(body.clone());
        let dir = temp_dir("skip");

        // Played and skipped, nobody else wants it: stopped, nothing cached.
        let skipped = req(&url, "skipped");
        let opened = open(&skipped, &dir, u64::MAX).unwrap();
        let progress = opened.progress.clone().unwrap();
        drop(opened);
        wait_done(&progress);
        assert!(progress.cancel.load(Ordering::Relaxed));
        assert!(!is_cached(&dir, "skipped", "flac"));

        // Played and skipped while the prefetcher also wants it: finishes and is cached.
        let kept = req(&url, "kept");
        let opened = open(&kept, &dir, u64::MAX).unwrap();
        let Fetch::Downloading((_, progress)) = fetch(&kept, &dir, u64::MAX, false, true).unwrap() else {
            panic!("expected to join the running download");
        };
        drop(opened);
        wait_done(&progress);
        assert!(progress.done.load(Ordering::Acquire));
        assert!(is_cached(&dir, "kept", "flac"));
        let _ = fs::remove_dir_all(&dir);
    }
}
