//! Media sources: local files and progressively downloaded HTTP streams.
//!
//! An HTTP stream is written to a file in the cache directory by a background thread while
//! the decoder reads behind it. Memory use therefore stays flat regardless of file size, and
//! a finished download doubles as a cache entry for the next play.

use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
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
            failed: Mutex::new(None),
            wake: (Mutex::new(()), Condvar::new()),
        }
    }

    fn notify(&self) {
        let _guard = self.wake.0.lock().unwrap();
        self.wake.1.notify_all();
    }

    fn wait(&self, timeout: Duration) {
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
        // Nobody is listening any more: stop an unfinished download.
        self.progress.cancel.store(true, Ordering::Relaxed);
        self.progress.notify();
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

fn cached_path(cache_dir: &Path, req: &TrackReq) -> Option<PathBuf> {
    if req.cache_key.is_empty() {
        return None;
    }
    let ext = if req.suffix.is_empty() { "bin".to_string() } else { sanitize(&req.suffix) };
    Some(cache_dir.join(format!("{}.{}", sanitize(&req.cache_key), ext)))
}

static NONCE: AtomicU64 = AtomicU64::new(0);

/// Open the media for a track: a cache hit, a local file, or a new progressive download.
pub fn open(req: &TrackReq, cache_dir: &Path, cache_limit: u64) -> Result<Opened, String> {
    let is_http = req.url.starts_with("http://") || req.url.starts_with("https://");
    if !is_http {
        let file = File::open(&req.url).map_err(|e| format!("{}: {e}", req.url))?;
        return Ok(Opened { source: Box::new(file), progress: None });
    }

    let final_path = cached_path(cache_dir, req);
    if let Some(path) = &final_path {
        if let Ok(file) = File::open(path) {
            // Touch the entry so the LRU cleanup keeps what is actually listened to.
            if let Ok(touch) = fs::OpenOptions::new().write(true).open(path) {
                let _ = touch.set_modified(std::time::SystemTime::now());
            }
            return Ok(Opened { source: Box::new(file), progress: None });
        }
    }

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_connect(Some(Duration::from_secs(10)))
        .timeout_recv_response(Some(Duration::from_secs(20)))
        .user_agent("Lumen")
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
        sanitize(if req.cache_key.is_empty() { "stream" } else { &req.cache_key }),
        NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let mut writer = File::create(&part).map_err(|e| format!("cache file: {e}"))?;
    let reader = File::open(&part).map_err(|e| format!("cache file: {e}"))?;

    let progress = Arc::new(Progress::new(total));
    let pump = progress.clone();
    let cache_dir = cache_dir.to_path_buf();
    std::thread::Builder::new()
        .name("lumen-download".into())
        .spawn(move || {
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
            if complete {
                pump.done.store(true, Ordering::Release);
                match &final_path {
                    Some(dest) if fs::rename(&part, dest).is_ok() => trim_cache(&cache_dir, cache_limit),
                    _ => {
                        let _ = fs::remove_file(&part);
                    }
                }
            } else {
                let cancelled = pump.cancel.load(Ordering::Relaxed);
                if !cancelled {
                    let msg = result.err().unwrap_or_else(|| "connection closed early".into());
                    *pump.failed.lock().unwrap() = Some(msg);
                }
                let _ = fs::remove_file(&part);
            }
            pump.notify();
        })
        .map_err(|e| format!("thread: {e}"))?;

    Ok(Opened {
        source: Box::new(StreamSource { file: reader, pos: 0, file_pos: 0, progress: progress.clone() }),
        progress: Some(progress),
    })
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
