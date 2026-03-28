use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crossbeam_queue::ArrayQueue;

fn extension_hint(path: &std::path::Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase())
}

pub fn prefers_ffmpeg(path: &std::path::Path) -> bool {
    match extension_hint(path).as_deref() {
        Some("wma" | "asf" | "dsf" | "dff" | "dsdiff") => true,
        _ => false,
    }
}

pub struct FfmpegDecodeOpts {
    pub path: PathBuf,
    pub output_rate: u32,
    pub start_seconds: f64,
}

pub fn spawn_ffmpeg_decoder(
    opts: FfmpegDecodeOpts,
    queue: Arc<ArrayQueue<f32>>,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let path_str = match opts.path.to_str() {
            Some(s) => s.to_string(),
            None => return,
        };
        if stop.load(Ordering::Relaxed) {
            return;
        }
        let child_res = Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-nostdin",
                "-ss",
                &format!("{:.3}", opts.start_seconds),
                "-i",
                &path_str,
                "-f",
                "f32le",
                "-ac",
                "2",
                "-ar",
                &opts.output_rate.to_string(),
                "-",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn();

        let mut child = match child_res {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut stdout = match child.stdout.take() {
            Some(s) => s,
            None => return,
        };

        let chunk_bytes = opts.output_rate as usize * 4 * 2 / 20;
        let mut buf = vec![0u8; chunk_bytes.max(4096)];

        loop {
            if stop.load(Ordering::Relaxed) {
                let _ = child.kill();
                break;
            }
            match std::io::Read::read(&mut stdout, &mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    for chunk in buf[..n].chunks_exact(4) {
                        let f = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                        while queue.is_full() && !stop.load(Ordering::Relaxed) {
                            thread::sleep(Duration::from_millis(1));
                        }
                        if stop.load(Ordering::Relaxed) {
                            break;
                        }
                        let _ = queue.push(f);
                    }
                }
                Err(_) => break,
            }
        }
        let _ = child.wait();
    })
}

/// Lightweight probe using ffprobe if available; falls back to None.
pub fn ffprobe_duration_ms(path: &std::path::Path) -> Option<u64> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            path.to_str()?,
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let sec: f64 = s.trim().parse().ok()?;
    Some((sec * 1000.0) as u64)
}
