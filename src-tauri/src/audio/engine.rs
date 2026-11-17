//! The engine thread: owns the decks, mixes, runs DSP and feeds the output ring.

use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, RecvTimeoutError, Sender};
use rtrb::{Producer, RingBuffer};

use super::backend::{OutCmd, OutShared, CHANNELS};
use super::deck::Deck;
use super::dsp::{DspChain, DspConfig};
use super::{
    source, DeviceFormat, DeviceInfo, EngineEvent, Options, OutputConfig, PlayState,
    ReplayGainMode, SignalPath, SourceInfo, TrackReq,
};

/// Ring capacity in samples: half a second of stereo at 768 kHz.
const RING_CAPACITY: usize = 768_000;
/// How much audio to keep queued ahead of the device. Small enough that EQ changes are
/// heard almost immediately, large enough to ride out scheduling hiccups.
const FILL_SECS: f64 = 0.25;
/// Frames decoded per mixing step.
const BLOCK: usize = 2048;
/// Time allowed for the device's own buffer to play out after the ring empties.
const TAIL: Duration = Duration::from_millis(150);
/// An exclusive device is handed back to the system after this long in pause.
const RELEASE_AFTER_PAUSE: Duration = Duration::from_secs(30);

pub enum Command {
    Play(TrackReq),
    SetNext(Option<TrackReq>),
    Pause,
    Resume,
    Stop,
    Seek(f64),
    SetVolume(f64),
    SetDsp(DspConfig),
    /// Whether a visualiser is on screen and wants spectrum frames.
    SetVisualizer(bool),
    SetOutput(OutputConfig),
    SetOptions(Options),
    ListDevices(Sender<Vec<DeviceInfo>>),
}

enum Msg {
    Cmd(Command),
    Loaded { generation: u64, next: bool, token: u64, result: Result<Box<Deck>, String> },
}

#[derive(Clone)]
pub struct EngineHandle {
    tx: Sender<Msg>,
}

impl EngineHandle {
    pub fn send(&self, cmd: Command) {
        let _ = self.tx.send(Msg::Cmd(cmd));
    }

    pub fn list_devices(&self) -> Vec<DeviceInfo> {
        let (tx, rx) = bounded(1);
        self.send(Command::ListDevices(tx));
        rx.recv_timeout(Duration::from_secs(3)).unwrap_or_default()
    }
}

/// A stretch of the output stream that belongs to one track.
struct Segment {
    serial: u64,
    /// Output frame (since the last flush) at which this track becomes audible.
    start: u64,
    token: u64,
    /// Track position, in seconds, at `start`.
    base: f64,
    info: SourceInfo,
    resampled: bool,
    gain: f64,
}

struct Crossfade {
    total: u64,
    done: u64,
}

type EventSink = Arc<dyn Fn(EngineEvent) + Send + Sync>;

pub struct Engine {
    rx: Receiver<Msg>,
    tx: Sender<Msg>,
    emit: EventSink,
    cache_dir: PathBuf,

    out: Sender<OutCmd>,
    shared: Arc<OutShared>,
    producer: Producer<f64>,
    /// Samples that did not fit in the ring on the last push.
    carry: Vec<f64>,
    block: Vec<f64>,

    out_cfg: OutputConfig,
    opts: Options,
    dsp: DspChain,
    dev: Option<DeviceFormat>,
    /// (device, exclusive, source rate) the current device was opened for.
    dev_key: Option<(Option<String>, bool, u32)>,

    cur: Option<Box<Deck>>,
    next: Option<Box<Deck>>,
    next_req: Option<TrackReq>,
    /// A `SetNext` that arrived while its slot was busy crossfading.
    queued_next: Option<Option<TrackReq>>,
    generation: u64,
    next_generation: u64,
    loading: Option<u64>,
    crossfade: Option<Crossfade>,

    paused: bool,
    paused_at: Option<Instant>,
    /// Position to return to after the device was released during a long pause.
    resume_at: Option<f64>,
    pending_seek: Option<f64>,
    drained_at: Option<Instant>,

    timeline: Vec<Segment>,
    serial: u64,
    frames_written: u64,
    audible: Option<u64>,
    last_status: Instant,
    last_state: PlayState,
    last_signal: Option<SignalPath>,
    viz: bool,
    analyzer: Option<super::analyzer::Analyzer>,
    last_viz: Instant,
}

impl Engine {
    /// Start the engine and its output backend. `emit` is called from the engine thread.
    pub fn spawn(cache_dir: PathBuf, emit: impl Fn(EngineEvent) + Send + Sync + 'static) -> EngineHandle {
        let (tx, rx) = crossbeam_channel::unbounded();
        let (producer, consumer) = RingBuffer::new(RING_CAPACITY);
        let shared = Arc::new(OutShared::new());
        let out = super::output::spawn(consumer, shared.clone());
        source::remove_stale_parts(&cache_dir);

        let engine = Engine {
            rx,
            tx: tx.clone(),
            emit: Arc::new(emit),
            cache_dir,
            out,
            shared,
            producer,
            carry: Vec::new(),
            block: Vec::new(),
            out_cfg: OutputConfig::default(),
            opts: Options::default(),
            dsp: DspChain::new(),
            dev: None,
            dev_key: None,
            cur: None,
            next: None,
            next_req: None,
            queued_next: None,
            generation: 0,
            next_generation: 0,
            loading: None,
            crossfade: None,
            paused: false,
            paused_at: None,
            resume_at: None,
            pending_seek: None,
            drained_at: None,
            timeline: Vec::new(),
            serial: 0,
            frames_written: 0,
            audible: None,
            last_status: Instant::now(),
            last_state: PlayState::Stopped,
            last_signal: None,
            viz: false,
            analyzer: None,
            last_viz: Instant::now(),
        };
        std::thread::Builder::new()
            .name("lumen-engine".into())
            .spawn(move || engine.run())
            .expect("spawn engine thread");
        EngineHandle { tx }
    }

    fn run(mut self) {
        let mut worked = false;
        loop {
            // While there is audio to produce, only peek at the queue; otherwise sleep on it.
            let wait = if worked { Duration::ZERO } else { Duration::from_millis(10) };
            match self.rx.recv_timeout(wait) {
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
            while let Ok(msg) = self.rx.try_recv() {
                self.handle(msg);
            }
            worked = self.pump();
            self.tick();
        }
    }

    // ---------------------------------------------------------------- commands

    fn handle(&mut self, msg: Msg) {
        match msg {
            Msg::Loaded { generation, next: false, token, result } => {
                if generation == self.generation {
                    self.on_loaded(token, result);
                }
            }
            Msg::Loaded { generation, next: true, token, result } => {
                if generation == self.next_generation {
                    match result {
                        Ok(deck) => self.next = Some(deck),
                        Err(message) => {
                            self.next_req = None;
                            self.event(EngineEvent::Error { token: Some(token), message });
                        }
                    }
                }
            }
            Msg::Cmd(cmd) => self.command(cmd),
        }
    }

    fn command(&mut self, cmd: Command) {
        match cmd {
            Command::Play(req) => {
                self.generation += 1;
                self.loading = Some(req.token);
                self.drop_decks();
                self.flush(false);
                self.paused = false;
                self.paused_at = None;
                self.load(req, false);
                self.status(true);
            }
            Command::SetNext(req) => self.set_next(req),
            Command::Pause => {
                if !self.paused && (self.cur.is_some() || self.loading.is_some()) {
                    self.paused = true;
                    self.paused_at = Some(Instant::now());
                    let (tx, rx) = bounded(1);
                    if self.out.send(OutCmd::Pause(tx)).is_ok() {
                        let _ = rx.recv_timeout(Duration::from_millis(500));
                    }
                    self.status(true);
                }
            }
            Command::Resume => {
                if self.paused {
                    self.paused = false;
                    self.paused_at = None;
                    if self.dev.is_none() && self.cur.is_some() {
                        let at = self.resume_at.take().unwrap_or(0.0);
                        self.reopen_at(at);
                    } else {
                        let _ = self.out.send(OutCmd::Start);
                    }
                    self.status(true);
                }
            }
            Command::Stop => self.stop(),
            Command::Seek(secs) => {
                if self.cur.is_some() {
                    if self.crossfade.is_some() {
                        self.finish_crossfade();
                    }
                    self.pending_seek = Some(secs.max(0.0));
                    self.status(true);
                }
            }
            Command::SetVolume(v) => self.shared.set_volume(v),
            Command::SetDsp(cfg) => self.dsp.set_config(cfg),
            Command::SetVisualizer(on) => {
                self.viz = on;
                if !on {
                    // Free the FFT buffers while nothing is shown.
                    self.analyzer = None;
                }
            }
            Command::SetOutput(cfg) => {
                if cfg != self.out_cfg {
                    self.out_cfg = cfg;
                    if self.cur.is_some() {
                        let at = self.resume_at.take().unwrap_or_else(|| self.position());
                        if self.paused {
                            // Stay released; the new device is opened on resume.
                            self.close_device();
                            self.resume_at = Some(at);
                        } else {
                            self.reopen_at(at);
                        }
                    } else {
                        self.close_device();
                    }
                }
            }
            Command::SetOptions(opts) => {
                let gain_changed = opts.replay_gain != self.opts.replay_gain
                    || opts.replay_gain_preamp_db != self.opts.replay_gain_preamp_db
                    || opts.prevent_clipping != self.opts.prevent_clipping;
                self.opts = opts;
                if gain_changed {
                    if let Some(deck) = &mut self.cur {
                        let g = replay_gain(&self.opts, &deck.req);
                        deck.set_gain(g);
                        if let Some(seg) = self.timeline.last_mut() {
                            seg.gain = g;
                        }
                    }
                }
            }
            Command::ListDevices(reply) => {
                let _ = self.out.send(OutCmd::ListDevices(reply));
            }
        }
    }

    fn set_next(&mut self, req: Option<TrackReq>) {
        if self.crossfade.is_some() {
            // The next slot is audible right now; apply this once the fade completes.
            self.queued_next = Some(req);
            return;
        }
        if req.as_ref().map(|r| r.token) == self.next_req.as_ref().map(|r| r.token) {
            return;
        }
        self.next_generation += 1;
        self.next = None;
        self.next_req = req.clone();
        if let Some(req) = req {
            self.load(req, true);
        }
    }

    fn load(&mut self, req: TrackReq, next: bool) {
        let generation = if next { self.next_generation } else { self.generation };
        let tx = self.tx.clone();
        let cache_dir = self.cache_dir.clone();
        let limit = self.opts.cache_limit_mb * 1024 * 1024;
        let token = req.token;
        let spawned = std::thread::Builder::new().name("lumen-loader".into()).spawn(move || {
            let result = Deck::open(req, &cache_dir, limit).map(Box::new);
            let _ = tx.send(Msg::Loaded { generation, next, token, result });
        });
        if let Err(e) = spawned {
            let _ = self.tx.send(Msg::Loaded {
                generation,
                next,
                token,
                result: Err(format!("could not start loader: {e}")),
            });
        }
    }

    fn on_loaded(&mut self, token: u64, result: Result<Box<Deck>, String>) {
        self.loading = None;
        let opened = result.and_then(|deck| {
            self.ensure_device(deck.info.rate)?;
            Ok(deck)
        });
        match opened.and_then(|deck| self.activate(deck, 0.0)) {
            Ok(()) => {
                if !self.paused {
                    let _ = self.out.send(OutCmd::Start);
                }
            }
            Err(message) => {
                self.event(EngineEvent::Error { token: Some(token), message });
                self.close_device();
            }
        }
        self.status(true);
    }

    fn stop(&mut self) {
        self.generation += 1;
        self.loading = None;
        self.drop_decks();
        self.flush(false);
        self.close_device();
        self.paused = false;
        self.paused_at = None;
        self.resume_at = None;
        self.status(true);
    }

    fn drop_decks(&mut self) {
        self.cur = None;
        self.next = None;
        self.next_req = None;
        self.queued_next = None;
        self.next_generation += 1;
        self.crossfade = None;
        self.pending_seek = None;
        self.drained_at = None;
    }

    // ------------------------------------------------------------------ device

    fn ensure_device(&mut self, source_rate: u32) -> Result<(), String> {
        let exclusive = self.out_cfg.exclusive;
        let key = (self.out_cfg.device_id.clone(), exclusive, if exclusive { source_rate } else { 0 });
        if self.dev.is_some() && self.dev_key.as_ref() == Some(&key) {
            return Ok(());
        }
        let (tx, rx) = bounded(1);
        self.out
            .send(OutCmd::Open {
                device_id: self.out_cfg.device_id.clone(),
                exclusive,
                rate: source_rate,
                reply: tx,
            })
            .map_err(|_| "audio output thread is gone".to_string())?;
        let format = rx
            .recv_timeout(Duration::from_secs(8))
            .map_err(|_| "audio device did not respond".to_string())??;
        self.shared.failed.store(false, Ordering::Release);
        self.dsp.set_rate(format.rate);
        self.dsp.reset();
        self.dev = Some(format);
        self.dev_key = Some(key);
        Ok(())
    }

    fn close_device(&mut self) {
        if self.dev.take().is_some() {
            let _ = self.out.send(OutCmd::Close);
        }
        self.dev_key = None;
    }

    /// (Re)open the device for the current deck and continue from `secs`.
    fn reopen_at(&mut self, secs: f64) {
        if self.crossfade.is_some() {
            self.finish_crossfade();
        }
        self.close_device();
        self.flush(true);
        let Some(rate) = self.cur.as_ref().map(|d| d.info.rate) else { return };
        let ready = self.ensure_device(rate).and_then(|()| {
            let out_rate = self.dev.as_ref().map_or(rate, |d| d.rate);
            self.cur.as_mut().map_or(Ok(()), |d| d.set_output_rate(out_rate))
        });
        match ready {
            Ok(()) => {
                self.pending_seek = Some(secs);
                if !self.paused {
                    let _ = self.out.send(OutCmd::Start);
                }
            }
            Err(message) => {
                let token = self.cur.as_ref().map(|d| d.req.token);
                self.event(EngineEvent::Error { token, message });
                self.stop();
            }
        }
    }

    /// Whether `deck` can follow the current audio without reopening the device.
    fn compatible(&self, deck: &Deck) -> bool {
        match (&self.dev, &self.dev_key) {
            (Some(dev), Some(key)) => !dev.exclusive || key.2 == deck.info.rate,
            _ => false,
        }
    }

    fn flush(&mut self, fade_in: bool) {
        let (tx, rx) = bounded(1);
        if self.out.send(OutCmd::Flush { fade_in, reply: tx }).is_ok() {
            let _ = rx.recv_timeout(Duration::from_millis(500));
        }
        self.carry.clear();
        self.timeline.clear();
        self.frames_written = 0;
        self.drained_at = None;
        self.dsp.reset();
    }

    // ------------------------------------------------------------------- decks

    /// Make `deck` the current deck, audible from the present end of the output stream.
    fn activate(&mut self, mut deck: Box<Deck>, base: f64) -> Result<(), String> {
        let rate = self.dev.as_ref().map(|d| d.rate).ok_or("no output device")?;
        deck.set_output_rate(rate)?;
        deck.set_gain(replay_gain(&self.opts, &deck.req));
        self.push_segment(&deck, base);
        self.cur = Some(deck);
        Ok(())
    }

    fn push_segment(&mut self, deck: &Deck, base: f64) {
        self.serial += 1;
        self.timeline.push(Segment {
            serial: self.serial,
            start: self.frames_written,
            token: deck.req.token,
            base,
            info: deck.info.clone(),
            resampled: deck.resampled(),
            gain: deck.gain(),
        });
    }

    fn finish_crossfade(&mut self) {
        self.crossfade = None;
        if let Some(next) = self.next.take() {
            self.cur = Some(next);
        }
        self.next_req = None;
        if let Some(queued) = self.queued_next.take() {
            self.set_next(queued);
        }
    }

    fn maybe_start_crossfade(&mut self) {
        if self.crossfade.is_some() || self.opts.crossfade_secs <= 0.0 {
            return;
        }
        let (Some(cur), Some(next), Some(dev)) = (&self.cur, &self.next, &self.dev) else { return };
        // Tracks of one album flow into each other; fading them would be wrong.
        let same_album = !cur.req.album_id.is_empty() && cur.req.album_id == next.req.album_id;
        if same_album || !self.compatible(next) || !next.ready() {
            return;
        }
        let Some(remaining) = cur.remaining_secs() else { return };
        if remaining > self.opts.crossfade_secs {
            return;
        }
        let rate = dev.rate;
        let total = (remaining.min(self.opts.crossfade_secs) * rate as f64) as u64;
        if total < rate as u64 / 10 {
            return;
        }
        let gain = replay_gain(&self.opts, &next.req);
        let next = self.next.as_mut().unwrap();
        if next.set_output_rate(rate).is_err() {
            return;
        }
        next.set_gain(gain);
        let next = self.next.take().unwrap();
        self.push_segment(&next, 0.0);
        self.next = Some(next);
        self.crossfade = Some(Crossfade { total, done: 0 });
    }

    /// The current deck has nothing more to give. Move on to the next one, or finish.
    fn advance(&mut self) -> bool {
        if let Some(message) = self.cur.as_mut().and_then(|d| d.error.take()) {
            let token = self.cur.as_ref().map(|d| d.req.token);
            self.event(EngineEvent::Error { token, message });
        }

        if self.next.as_ref().is_some_and(|n| self.compatible(n)) {
            // Gapless: the next track's first sample follows the last one of this track.
            let next = self.next.take().unwrap();
            self.next_req = None;
            return match self.activate(next, 0.0) {
                Ok(()) => true,
                Err(message) => {
                    self.event(EngineEvent::Error { token: None, message });
                    self.cur = None;
                    false
                }
            };
        }

        if !self.drained() {
            return false;
        }
        if let Some(next) = self.next.take() {
            // Different sample rate in exclusive mode: the device has to be reopened.
            self.next_req = None;
            self.cur = None;
            self.close_device();
            self.flush(false);
            let token = next.req.token;
            let ready = self.ensure_device(next.info.rate).and_then(|()| self.activate(next, 0.0));
            match ready {
                Ok(()) => {
                    if !self.paused {
                        let _ = self.out.send(OutCmd::Start);
                    }
                }
                Err(message) => self.event(EngineEvent::Error { token: Some(token), message }),
            }
            return false;
        }
        if self.next_req.is_some() {
            // The next track is still loading; keep waiting.
            return false;
        }
        self.cur = None;
        self.flush(false);
        self.close_device();
        self.event(EngineEvent::Ended);
        self.status(true);
        false
    }

    /// True once everything queued has left the ring and the device buffer has played out.
    fn drained(&mut self) -> bool {
        let empty = self.carry.is_empty()
            && self.shared.frames_played.load(Ordering::Acquire) >= self.frames_written;
        if !empty {
            self.drained_at = None;
            return false;
        }
        let since = *self.drained_at.get_or_insert_with(Instant::now);
        since.elapsed() >= TAIL
    }

    // -------------------------------------------------------------------- pump

    /// Produce one block of audio if the ring has room. Returns true when it did.
    fn pump(&mut self) -> bool {
        if self.shared.failed.swap(false, Ordering::AcqRel) && self.cur.is_some() {
            // The device vanished (unplugged, driver reset). Pick up where we were.
            let at = self.position();
            self.dev = None;
            self.dev_key = None;
            self.reopen_at(at);
            return false;
        }
        if let Some(secs) = self.pending_seek {
            if !self.try_seek(secs) {
                return false;
            }
        }
        if self.paused {
            return false;
        }
        let Some(rate) = self.dev.as_ref().map(|d| d.rate) else { return false };
        if !self.push_carry() {
            return false;
        }
        let target = (rate as f64 * FILL_SECS) as usize * CHANNELS;
        if RING_CAPACITY - self.producer.slots() >= target {
            return false;
        }
        let Some(cur) = self.cur.as_mut() else { return false };
        cur.fill(BLOCK);
        self.maybe_start_crossfade();

        let mut block = std::mem::take(&mut self.block);
        block.clear();
        let cur = self.cur.as_mut().unwrap();

        if let Some(fade) = &mut self.crossfade {
            let next = self.next.as_mut().expect("crossfade without next deck");
            next.fill(BLOCK);
            let frames = (cur.fifo.len().min(next.fifo.len()) / CHANNELS)
                .min((fade.total - fade.done) as usize);
            if frames == 0 {
                let finished = fade.done >= fade.total
                    || (cur.eof && cur.fifo.is_empty())
                    || (next.eof && next.fifo.is_empty());
                self.block = block;
                if finished {
                    self.finish_crossfade();
                }
                return finished;
            }
            // Equal-power curves keep the perceived loudness steady through the fade.
            for i in 0..frames {
                let p = (fade.done + i as u64) as f64 / fade.total as f64;
                let (g_in, g_out) = (p * std::f64::consts::FRAC_PI_2).sin_cos();
                for c in 0..CHANNELS {
                    let k = i * CHANNELS + c;
                    block.push(cur.fifo[k] * g_out + next.fifo[k] * g_in);
                }
            }
            cur.fifo.drain(..frames * CHANNELS);
            next.fifo.drain(..frames * CHANNELS);
            fade.done += frames as u64;
        } else {
            if cur.fifo.is_empty() {
                self.block = block;
                return if cur.eof { self.advance() } else { false };
            }
            std::mem::swap(&mut block, &mut cur.fifo);
        }

        if self.dsp.is_active() {
            self.dsp.process(&mut block);
        }
        self.frames_written += (block.len() / CHANNELS) as u64;
        self.drained_at = None;
        self.carry.extend_from_slice(&block);
        self.block = block;
        self.push_carry();
        true
    }

    /// Move carried samples into the ring. Returns true when nothing is left over.
    fn push_carry(&mut self) -> bool {
        if self.carry.is_empty() {
            return true;
        }
        let n = self.carry.len().min(self.producer.slots());
        // Keep frames whole.
        let n = n - n % CHANNELS;
        if n > 0 {
            if let Ok(mut chunk) = self.producer.write_chunk(n) {
                let (a, b) = chunk.as_mut_slices();
                a.copy_from_slice(&self.carry[..a.len()]);
                b.copy_from_slice(&self.carry[a.len()..n]);
                chunk.commit_all();
                self.carry.drain(..n);
            }
        }
        self.carry.is_empty()
    }

    fn try_seek(&mut self, secs: f64) -> bool {
        let Some(cur) = self.cur.as_mut() else {
            self.pending_seek = None;
            return true;
        };
        if !cur.can_seek(secs) {
            // Not downloaded that far yet; keep playing and retry.
            return false;
        }
        self.pending_seek = None;
        let result = self.cur.as_mut().unwrap().seek(secs);
        self.flush(true);
        match result {
            Ok(actual) => {
                let deck = self.cur.take().unwrap();
                self.push_segment(&deck, actual);
                self.cur = Some(deck);
                // Same track, new position: not a track change.
                self.audible = Some(self.serial);
            }
            Err(message) => {
                let token = self.cur.as_ref().map(|d| d.req.token);
                self.event(EngineEvent::Error { token, message });
            }
        }
        true
    }

    // ------------------------------------------------------------------ status

    fn current_segment(&self) -> Option<&Segment> {
        let played = self.shared.frames_played.load(Ordering::Acquire);
        self.timeline.iter().rev().find(|s| s.start <= played).or(self.timeline.first())
    }

    fn position(&self) -> f64 {
        if let Some(secs) = self.pending_seek.or(self.resume_at) {
            return secs;
        }
        let (Some(seg), Some(dev)) = (self.current_segment(), &self.dev) else { return 0.0 };
        let played = self.shared.frames_played.load(Ordering::Acquire);
        let pos = seg.base + played.saturating_sub(seg.start) as f64 / dev.rate as f64;
        seg.info.duration.map_or(pos, |d| pos.min(d))
    }

    fn state(&self) -> PlayState {
        if self.loading.is_some() {
            return if self.paused { PlayState::Paused } else { PlayState::Loading };
        }
        let Some(cur) = &self.cur else { return PlayState::Stopped };
        if self.paused {
            return PlayState::Paused;
        }
        let starved = self.carry.is_empty()
            && self.shared.frames_played.load(Ordering::Acquire) >= self.frames_written
            && !cur.eof;
        if starved && (!cur.ready() || self.frames_written == 0) {
            PlayState::Buffering
        } else {
            PlayState::Playing
        }
    }

    fn tick(&mut self) {
        // Announce the track that just became audible.
        if let Some((serial, token)) = self.current_segment().map(|s| (s.serial, s.token)) {
            if self.audible != Some(serial) {
                self.audible = Some(serial);
                self.event(EngineEvent::TrackStarted { token });
                self.status(true);
            }
            // Segments behind the audible one are history.
            self.timeline.retain(|s| s.serial >= serial);
        }

        if let Some(since) = self.paused_at {
            let exclusive = self.dev.as_ref().is_some_and(|d| d.exclusive);
            if exclusive && since.elapsed() > RELEASE_AFTER_PAUSE {
                // Do not hold the sound card hostage while paused.
                let at = self.position();
                self.close_device();
                self.flush(true);
                self.resume_at = Some(at);
                self.paused_at = None;
            }
        }

        self.update_signal();
        self.visualize();
        self.status(false);
    }

    fn visualize(&mut self) {
        if !self.viz || self.paused || self.cur.is_none() || self.last_viz.elapsed() < Duration::from_millis(33) {
            return;
        }
        let Some(rate) = self.dev.as_ref().map(|d| d.rate) else { return };
        self.last_viz = Instant::now();
        let analyzer = self.analyzer.get_or_insert_with(super::analyzer::Analyzer::new);
        let frame = {
            let tap = self.shared.tap.lock();
            analyzer.analyze(&tap, rate)
        };
        self.event(EngineEvent::Viz(frame));
    }

    fn update_signal(&mut self) {
        let path = match (self.current_segment(), &self.dev) {
            (Some(seg), Some(dev)) => {
                let dsp_active = self.dsp.is_active();
                let volume_unity = self.shared.volume() == 1.0;
                let processed = seg.resampled
                    || seg.gain != 1.0
                    || dsp_active
                    || self.crossfade.is_some()
                    || seg.info.channels > 2;
                let depth_fits = seg.info.bits.is_some_and(|b| b <= dev.bits as u32);
                let exact_source = seg.info.lossless && depth_fits;
                // Dither whenever the integer output is not an exact copy of the source.
                let dither = !dev.float && (processed || !exact_source);
                self.shared.dither.store(dither, Ordering::Relaxed);
                Some(SignalPath {
                    source: seg.info.clone(),
                    device: dev.clone(),
                    resampled: seg.resampled,
                    replay_gain_db: 20.0 * seg.gain.log10(),
                    dsp_active,
                    volume_unity,
                    dithered: !dev.float && (dither || !volume_unity),
                    bit_perfect: dev.exclusive && !dev.float && exact_source && !processed && volume_unity,
                })
            }
            _ => None,
        };
        if path != self.last_signal {
            self.last_signal = path.clone();
            self.event(EngineEvent::Signal { path });
        }
    }

    fn status(&mut self, force: bool) {
        let state = self.state();
        if !force && state == self.last_state && self.last_status.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.last_state = state;
        self.last_status = Instant::now();
        let seg = self.current_segment();
        let token = seg.map(|s| s.token).or(self.loading);
        let duration = seg.and_then(|s| s.info.duration).unwrap_or(0.0);
        let buffered = self.cur.as_ref().map_or(0.0, |d| d.buffered_fraction());
        let position = self.position();
        self.event(EngineEvent::Status { state, token, position, duration, buffered });
    }

    fn event(&self, event: EngineEvent) {
        (self.emit)(event);
    }
}

/// Linear gain for a track under the current ReplayGain settings. 1.0 means untouched.
fn replay_gain(opts: &Options, req: &TrackReq) -> f64 {
    let rg = &req.replay_gain;
    let (gain_db, peak) = match opts.replay_gain {
        ReplayGainMode::Off => return 1.0,
        ReplayGainMode::Track => (rg.track_gain.or(rg.album_gain), rg.track_peak.or(rg.album_peak)),
        ReplayGainMode::Album => (rg.album_gain.or(rg.track_gain), rg.album_peak.or(rg.track_peak)),
    };
    // Untagged files are left alone rather than guessed at.
    let Some(gain_db) = gain_db else { return 1.0 };
    let mut gain = 10f64.powf((gain_db + opts.replay_gain_preamp_db) / 20.0);
    if opts.prevent_clipping {
        if let Some(peak) = peak.filter(|p| *p > 0.0) {
            gain = gain.min(1.0 / peak);
        }
    }
    gain
}

#[cfg(test)]
mod tests {
    use super::super::ReplayGainInfo;
    use super::*;

    fn req(rg: ReplayGainInfo) -> TrackReq {
        TrackReq {
            token: 1,
            url: String::new(),
            cache_key: String::new(),
            suffix: String::new(),
            duration: 0.0,
            album_id: String::new(),
            replay_gain: rg,
        }
    }

    #[test]
    fn replay_gain_off_or_untagged_is_exactly_unity() {
        let tagged = ReplayGainInfo { track_gain: Some(-6.0), ..Default::default() };
        assert_eq!(replay_gain(&Options::default(), &req(tagged)), 1.0);
        let on = Options { replay_gain: ReplayGainMode::Track, ..Default::default() };
        assert_eq!(replay_gain(&on, &req(ReplayGainInfo::default())), 1.0);
    }

    #[test]
    fn replay_gain_applies_gain_and_respects_peak() {
        let on = Options { replay_gain: ReplayGainMode::Track, ..Default::default() };
        let g = replay_gain(&on, &req(ReplayGainInfo { track_gain: Some(-6.0), ..Default::default() }));
        assert!((g - 0.501).abs() < 0.001);

        // +6 dB would clip a track that already peaks at 0.9.
        let hot = ReplayGainInfo { track_gain: Some(6.0), track_peak: Some(0.9), ..Default::default() };
        let g = replay_gain(&on, &req(hot));
        assert!((g - 1.0 / 0.9).abs() < 1e-9);

        let album = Options { replay_gain: ReplayGainMode::Album, ..Default::default() };
        let both = ReplayGainInfo { track_gain: Some(-3.0), album_gain: Some(-9.0), ..Default::default() };
        assert!((replay_gain(&album, &req(both)) - 10f64.powf(-9.0 / 20.0)).abs() < 1e-9);
    }
}
