use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_queue::ArrayQueue;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;

use symphonia::core::audio::{AudioBufferRef, SampleBuffer};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error as SymphError;
use symphonia::core::formats::{FormatOptions, SeekMode, SeekTo};
use symphonia::core::units::Time;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

use crate::audio::ffmpeg_cli::{prefers_ffmpeg, spawn_ffmpeg_decoder, FfmpegDecodeOpts};
use crate::audio::probe::symphonia_can_decode;

const QUEUE_CAP: usize = 48000 * 2 * 8;

#[derive(Default, Clone, serde::Serialize)]
pub struct PlaybackStatus {
    pub path: Option<String>,
    pub decoder: Option<String>,
    pub duration_ms: u64,
    pub position_ms: u64,
    pub playing: bool,
    pub volume: f32,
}

pub struct PlaybackEngine {
    queue: Arc<ArrayQueue<f32>>,
    status: Arc<Mutex<PlaybackStatus>>,
    playing: Arc<AtomicBool>,
    stop_decode: Arc<AtomicBool>,
    volume: Arc<Mutex<f32>>,
    decode_handle: Mutex<Option<JoinHandle<()>>>,
    /// cpal output stream is !Send on Windows; it lives only in this thread.
    _audio_thread: JoinHandle<()>,
    output_sample_rate: u32,
    #[allow(dead_code)]
    timing: Mutex<(Instant, u64)>,
    samples_sent: Arc<AtomicU64>,
    base_position_ms: Arc<AtomicU64>,
}

impl PlaybackEngine {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "no audio output device".to_string())?;
        let supported = device
            .default_output_config()
            .map_err(|e| e.to_string())?;
        let range = supported.config();
        let output_sample_rate = range.sample_rate.0;
        let channels = range.channels as usize;
        if channels < 2 {
            return Err("mono output not supported".to_string());
        }
        let queue = Arc::new(ArrayQueue::new(QUEUE_CAP));
        let status = Arc::new(Mutex::new(PlaybackStatus {
            volume: 1.0,
            ..Default::default()
        }));
        let playing_flag = Arc::new(AtomicBool::new(false));
        let stop_decode = Arc::new(AtomicBool::new(false));
        let volume = Arc::new(Mutex::new(1.0f32));
        let samples_sent = Arc::new(AtomicU64::new(0));
        let base_position_ms = Arc::new(AtomicU64::new(0));

        let q_cb = Arc::clone(&queue);
        let vol_cb = Arc::clone(&volume);
        let playing_cb = Arc::clone(&playing_flag);
        let samples_cb = Arc::clone(&samples_sent);
        let base_cb = Arc::clone(&base_position_ms);
        let status_cb = Arc::clone(&status);
        let sr = output_sample_rate;

        let audio_thread = thread::spawn(move || {
            let cfg = cpal::StreamConfig {
                channels: 2,
                sample_rate: cpal::SampleRate(sr),
                buffer_size: cpal::BufferSize::Default,
            };
            let host = cpal::default_host();
            let Some(device) = host.default_output_device() else {
                return;
            };
            let Ok(stream) = device.build_output_stream(
                &cfg,
                move |data: &mut [f32], _| {
                    let vol = *vol_cb.lock();
                    let play = playing_cb.load(Ordering::Relaxed);
                    for s in data.iter_mut() {
                        *s = if play {
                            q_cb.pop().unwrap_or(0.0) * vol
                        } else {
                            0.0
                        };
                    }
                    let pushed = data.len() as u64;
                    let prev = samples_cb.fetch_add(pushed, Ordering::Relaxed);
                    let total = prev + pushed;
                    let base = base_cb.load(Ordering::Relaxed);
                    let pos_ms = base + (total / 2 * 1000 / sr as u64);
                    status_cb.lock().position_ms = pos_ms;
                },
                |_| {},
                None,
            ) else {
                return;
            };
            if stream.play().is_err() {
                return;
            }
            loop {
                thread::sleep(Duration::from_secs(3600));
            }
        });

        Ok(Self {
            queue,
            status,
            playing: playing_flag,
            stop_decode,
            volume,
            decode_handle: Mutex::new(None),
            _audio_thread: audio_thread,
            output_sample_rate,
            timing: Mutex::new((Instant::now(), 0)),
            samples_sent,
            base_position_ms,
        })
    }

    fn stop_decode_thread(&self) {
        self.stop_decode.store(true, Ordering::SeqCst);
        if let Some(h) = self.decode_handle.lock().take() {
            let _ = h.join();
        }
        self.stop_decode.store(false, Ordering::SeqCst);
        while self.queue.pop().is_some() {}
    }

    pub fn load(self: &Arc<Self>, path: String) -> Result<(), String> {
        self.stop_decode_thread();
        self.playing.store(false, Ordering::SeqCst);
        self.samples_sent.store(0, Ordering::SeqCst);
        {
            let mut st = self.status.lock();
            st.path = Some(path.clone());
            st.decoder = None;
            st.duration_ms = 0;
            st.position_ms = 0;
            st.playing = false;
        }
        self.base_position_ms.store(0, Ordering::SeqCst);
        *self.timing.lock() = (Instant::now(), 0);

        let path_buf = PathBuf::from(&path);
        let sym_ok = symphonia_can_decode(&path_buf).is_ok();
        let use_ffmpeg = prefers_ffmpeg(&path_buf) || !sym_ok;

        let mut duration_ms = crate::audio::probe::probe_file(&path_buf)
            .ok()
            .and_then(|i| i.duration_ms)
            .unwrap_or(0);
        if duration_ms == 0 {
            if let Some(d) = crate::audio::ffmpeg_cli::ffprobe_duration_ms(&path_buf) {
                duration_ms = d;
            }
        }

        {
            let mut st = self.status.lock();
            st.duration_ms = duration_ms;
            st.decoder = Some(if use_ffmpeg {
                "ffmpeg".into()
            } else {
                "symphonia".into()
            });
        }

        let q = Arc::clone(&self.queue);
        let stop = Arc::clone(&self.stop_decode);
        let playing_flag = Arc::clone(&self.playing);
        let sr = self.output_sample_rate;

        let handle = if use_ffmpeg {
            spawn_ffmpeg_decoder(
                FfmpegDecodeOpts {
                    path: path_buf.clone(),
                    output_rate: sr,
                    start_seconds: 0.0,
                },
                Arc::clone(&q),
                Arc::clone(&stop),
            )
        } else {
            thread::spawn(move || {
                symphonia_decode_loop(path_buf, sr, q, stop, playing_flag);
            })
        };

        *self.decode_handle.lock() = Some(handle);
        Ok(())
    }

    pub fn play(self: &Arc<Self>) -> Result<(), String> {
        {
            let mut st = self.status.lock();
            if st.path.is_none() {
                return Err("nothing loaded".to_string());
            }
            st.playing = true;
        }
        let cur = self.status.lock().position_ms;
        self.base_position_ms.store(cur, Ordering::SeqCst);
        self.samples_sent.store(0, Ordering::SeqCst);
        self.playing.store(true, Ordering::SeqCst);
        Ok(())
    }

    pub fn pause(&self) {
        self.playing.store(false, Ordering::SeqCst);
        let pos = {
            let st = self.status.lock();
            st.position_ms
        };
        self.base_position_ms.store(pos, Ordering::SeqCst);
        self.samples_sent.store(0, Ordering::SeqCst);
        self.status.lock().playing = false;
    }

    pub fn seek(self: &Arc<Self>, position_ms: u64) -> Result<(), String> {
        let path = {
            let st = self.status.lock();
            st.path.clone().ok_or_else(|| "nothing loaded".to_string())?
        };
        let duration = self.status.lock().duration_ms;
        let pos = position_ms.min(duration);
        self.stop_decode_thread();
        self.playing.store(false, Ordering::SeqCst);
        self.samples_sent.store(0, Ordering::SeqCst);
        self.base_position_ms.store(pos, Ordering::SeqCst);
        {
            let mut st = self.status.lock();
            st.position_ms = pos;
        }

        let path_buf = Path::new(&path).to_path_buf();
        let sym_ok = symphonia_can_decode(&path_buf).is_ok();
        let use_ffmpeg = prefers_ffmpeg(&path_buf) || !sym_ok;
        let q = Arc::clone(&self.queue);
        let stop = Arc::clone(&self.stop_decode);
        let playing_flag = Arc::clone(&self.playing);
        let sr = self.output_sample_rate;
        let start_sec = pos as f64 / 1000.0;

        let handle = if use_ffmpeg {
            spawn_ffmpeg_decoder(
                FfmpegDecodeOpts {
                    path: path_buf,
                    output_rate: sr,
                    start_seconds: start_sec,
                },
                Arc::clone(&q),
                Arc::clone(&stop),
            )
        } else {
            thread::spawn(move || {
                symphonia_decode_loop_from_ms(path_buf, sr, q, stop, playing_flag, pos);
            })
        };
        *self.decode_handle.lock() = Some(handle);
        Ok(())
    }

    pub fn set_volume(&self, v: f32) {
        let v = v.clamp(0.0, 1.0);
        *self.volume.lock() = v;
        self.status.lock().volume = v;
    }

    pub fn status_snapshot(&self) -> PlaybackStatus {
        self.status.lock().clone()
    }
}

fn symphonia_decode_loop(
    path: PathBuf,
    out_rate: u32,
    q: Arc<ArrayQueue<f32>>,
    stop: Arc<AtomicBool>,
    playing: Arc<AtomicBool>,
) {
    symphonia_decode_loop_from_ms(path, out_rate, q, stop, playing, 0);
}

fn symphonia_decode_loop_from_ms(
    path: PathBuf,
    out_rate: u32,
    q: Arc<ArrayQueue<f32>>,
    stop: Arc<AtomicBool>,
    playing: Arc<AtomicBool>,
    start_ms: u64,
) {
    let _ = symphonia_decode_inner(&path, out_rate, &q, &stop, &playing, start_ms);
}

fn symphonia_decode_inner(
    path: &Path,
    out_rate: u32,
    q: &Arc<ArrayQueue<f32>>,
    stop: &Arc<AtomicBool>,
    playing: &Arc<AtomicBool>,
    start_ms: u64,
) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;
    let mut format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "no default audio track".to_string())?;
    let track_id = track.id;
    let dec_opts = DecoderOptions::default();
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map_err(|e| e.to_string())?;

    let in_rate = track.codec_params.sample_rate.unwrap_or(out_rate);
    if start_ms > 0 {
        let t = Time::from(start_ms as f64 / 1000.0);
        let _ = format.seek(
            SeekMode::Accurate,
            SeekTo::Time {
                time: t,
                track_id: Some(track_id),
            },
        );
        decoder.reset();
    }

    while !stop.load(Ordering::Relaxed) {
        if !playing.load(Ordering::Relaxed) {
            thread::sleep(std::time::Duration::from_millis(12));
            continue;
        }
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(SymphError::DecodeError(_)) => continue,
            Err(SymphError::ResetRequired) => {
                decoder.reset();
                continue;
            }
            Err(_) => continue,
        };
        let interleaved = match decode_to_interleaved_f32(decoded) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ch = interleaved.channels.max(1);
        let stereo = if ch == 1 {
            mono_to_stereo(&interleaved.data)
        } else {
            take_stereo(&interleaved.data, ch)
        };
        let resampled = if in_rate == out_rate {
            stereo
        } else {
            resample_stereo_linear(&stereo, in_rate, out_rate)
        };
        for chunk in resampled.chunks(512) {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            while !playing.load(Ordering::Relaxed) {
                thread::sleep(std::time::Duration::from_millis(8));
                if stop.load(Ordering::Relaxed) {
                    break;
                }
            }
            for &s in chunk {
                while q.len() > QUEUE_CAP.saturating_sub(256) && !stop.load(Ordering::Relaxed) {
                    thread::sleep(std::time::Duration::from_millis(2));
                }
                if stop.load(Ordering::Relaxed) {
                    break;
                }
                let _ = q.push(s);
            }
        }
    }
    Ok(())
}

struct InterleavedF32 {
    data: Vec<f32>,
    channels: usize,
}

fn decode_to_interleaved_f32(buf: AudioBufferRef<'_>) -> Result<InterleavedF32, String> {
    let spec = *buf.spec();
    let channels = spec.channels.count();
    let duration = buf.capacity() as u64;
    let mut sample_buf = SampleBuffer::<f32>::new(duration, spec);
    sample_buf.copy_interleaved_ref(buf);
    Ok(InterleavedF32 {
        data: sample_buf.samples().to_vec(),
        channels,
    })
}

fn mono_to_stereo(m: &[f32]) -> Vec<f32> {
    let mut v = Vec::with_capacity(m.len() * 2);
    for &s in m {
        v.push(s);
        v.push(s);
    }
    v
}

fn take_stereo(data: &[f32], ch: usize) -> Vec<f32> {
    if ch == 2 {
        return data.to_vec();
    }
    let frames = data.len() / ch;
    let mut v = Vec::with_capacity(frames * 2);
    for f in 0..frames {
        v.push(data[f * ch]);
        v.push(data[f * ch + 1.min(ch - 1)]);
    }
    v
}

fn resample_stereo_linear(input: &[f32], in_rate: u32, out_rate: u32) -> Vec<f32> {
    if in_rate == out_rate || input.is_empty() {
        return input.to_vec();
    }
    let in_frames = input.len() / 2;
    let ratio = in_rate as f64 / out_rate as f64;
    let out_frames = ((in_frames as f64) / ratio).floor().max(1.0) as usize;
    let mut out = Vec::with_capacity(out_frames * 2);
    for o in 0..out_frames {
        let src_pos = o as f64 * ratio;
        let i0 = src_pos.floor() as usize;
        let frac = (src_pos - i0 as f64) as f32;
        let i1 = (i0 + 1).min(in_frames.saturating_sub(1));
        for ch in 0..2 {
            let s0 = input[i0 * 2 + ch];
            let s1 = input[i1 * 2 + ch];
            out.push(s0 + (s1 - s0) * frac);
        }
    }
    out
}
