use std::fs::File;
use std::path::Path;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, Clone)]
pub struct ProbedInfo {
    pub sample_rate: Option<u32>,
    pub channels: Option<u8>,
    pub duration_ms: Option<u64>,
    pub codec_hint: Option<String>,
    pub container: Option<String>,
}

pub fn probe_file(path: &Path) -> Result<ProbedInfo, String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;

    let format = &probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "no default audio track".to_string())?;
    let codec_params = &track.codec_params;

    let sample_rate = codec_params.sample_rate;
    let channels = codec_params.channels.map(|c| c.count() as u8);

    let duration_ms = codec_params
        .time_base
        .zip(codec_params.n_frames)
        .and_then(|(tb, frames)| {
            let ts = tb.calc_time(frames);
            ts.seconds
                .checked_mul(1000)
                .and_then(|ms| ms.checked_add((ts.frac * 1000.0) as u64))
        });

    let codec_hint = Some(format!("{:?}", codec_params.codec));

    let container = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_ascii_lowercase());

    Ok(ProbedInfo {
        sample_rate,
        channels,
        duration_ms,
        codec_hint,
        container,
    })
}

/// Opens decoder to verify the stream is decodable (Symphonia path).
pub fn symphonia_can_decode(path: &Path) -> Result<(), String> {
    let file = File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;
    let format = probed.format;
    let track = format
        .default_track()
        .ok_or_else(|| "no default audio track".to_string())?;
    let dec_opts = DecoderOptions::default();
    symphonia::default::get_codecs()
        .make(&track.codec_params, &dec_opts)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
