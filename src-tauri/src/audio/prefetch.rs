//! Background caching of the play queue, so skipping and seeking never wait for the network.
//!
//! The UI hands over the upcoming tracks in play order; a couple of worker threads download
//! them into the stream cache one by one. A new list replaces the old one, and downloads of
//! tracks that left the queue are stopped (unless a player is reading them).

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use serde::Serialize;

use super::source::{self, Fetch, Progress};
use super::TrackReq;

const WORKERS: usize = 2;
/// Never let one queue fill more than this share of the cache, or later tracks would evict
/// the earlier ones they were downloaded alongside.
const BUDGET_SHARE: f64 = 0.8;

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum CacheEvent {
    #[serde(rename_all = "camelCase")]
    Cached { key: String },
    #[serde(rename_all = "camelCase")]
    Failed { key: String, message: String },
    #[serde(rename_all = "camelCase")]
    Status {
        done: usize,
        total: usize,
        active: Vec<String>,
        /// True when the size limit stopped the queue from being cached completely.
        limited: bool,
    },
}

struct State {
    queue: VecDeque<TrackReq>,
    generation: u64,
    limit: u64,
    used: u64,
    done: usize,
    total: usize,
    limited: bool,
    running: HashMap<String, Arc<Progress>>,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
    cache_dir: PathBuf,
    emit: Box<dyn Fn(CacheEvent) + Send + Sync>,
}

impl Shared {
    fn status(&self, s: &State) {
        (self.emit)(CacheEvent::Status {
            done: s.done,
            total: s.total,
            active: s.running.keys().cloned().collect(),
            limited: s.limited,
        });
    }
}

#[derive(Clone)]
pub struct Prefetcher {
    shared: Arc<Shared>,
}

impl Prefetcher {
    pub fn spawn(cache_dir: PathBuf, emit: impl Fn(CacheEvent) + Send + Sync + 'static) -> Prefetcher {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                queue: VecDeque::new(),
                generation: 0,
                limit: 0,
                used: 0,
                done: 0,
                total: 0,
                limited: false,
                running: HashMap::new(),
            }),
            wake: Condvar::new(),
            cache_dir,
            emit: Box::new(emit),
        });
        for i in 0..WORKERS {
            let shared = shared.clone();
            let _ = std::thread::Builder::new()
                .name(format!("lumen-prefetch-{i}"))
                .spawn(move || worker(shared));
        }
        Prefetcher { shared }
    }

    /// Replace the list of tracks to cache. An empty list stops prefetching.
    pub fn set(&self, tracks: Vec<TrackReq>, limit: u64) {
        let wanted: Vec<TrackReq> = tracks
            .into_iter()
            .filter(|t| !t.cache_key.is_empty() && (t.url.starts_with("http://") || t.url.starts_with("https://")))
            .collect();
        let mut s = self.shared.state.lock().unwrap();
        // Downloads for tracks that are no longer queued are not worth finishing.
        for (key, progress) in &s.running {
            if !wanted.iter().any(|t| &t.cache_key == key) {
                progress.release_keep();
            }
        }
        s.generation += 1;
        s.limit = limit;
        s.used = 0;
        s.done = 0;
        s.limited = false;
        s.total = wanted.len();
        s.queue = wanted.into();
        self.shared.wake.notify_all();
        self.shared.status(&s);
    }
}

fn worker(shared: Arc<Shared>) {
    loop {
        let (req, generation, limit) = {
            let mut s = shared.state.lock().unwrap();
            while s.queue.is_empty() {
                s = shared.wake.wait(s).unwrap();
            }
            let req = s.queue.pop_front().unwrap();
            (req, s.generation, s.limit)
        };

        let result = cache_one(&shared, &req, limit);

        let mut s = shared.state.lock().unwrap();
        s.running.remove(&req.cache_key);
        if s.generation != generation {
            // The queue changed meanwhile; this result belongs to an old list.
            shared.status(&s);
            continue;
        }
        s.done += 1;
        match result {
            Ok(bytes) => {
                s.used += bytes;
                (shared.emit)(CacheEvent::Cached { key: req.cache_key.clone() });
                if s.limit > 0 && s.used as f64 > s.limit as f64 * BUDGET_SHARE && !s.queue.is_empty() {
                    s.limited = true;
                    s.queue.clear();
                }
            }
            Err(message) => (shared.emit)(CacheEvent::Failed { key: req.cache_key.clone(), message }),
        }
        shared.status(&s);
    }
}

/// Make sure one track is fully cached. Returns its size.
fn cache_one(shared: &Shared, req: &TrackReq, limit: u64) -> Result<u64, String> {
    match source::fetch(req, &shared.cache_dir, limit, false, true)? {
        Fetch::Cached(path) => {
            source::touch(&path);
            Ok(std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0))
        }
        Fetch::Downloading((_, progress)) => {
            {
                let mut s = shared.state.lock().unwrap();
                s.running.insert(req.cache_key.clone(), progress.clone());
                shared.status(&s);
            }
            loop {
                if progress.done.load(Ordering::Acquire) {
                    return Ok(progress.downloaded.load(Ordering::Acquire));
                }
                if let Some(e) = progress.error() {
                    return Err(e);
                }
                if progress.cancel.load(Ordering::Relaxed) {
                    return Err("cancelled".into());
                }
                progress.wait(Duration::from_millis(250));
            }
        }
    }
}
