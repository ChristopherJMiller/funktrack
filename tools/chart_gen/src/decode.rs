use std::path::Path;

use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

pub struct AudioData {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

const TARGET_SAMPLE_RATE: u32 = 44100;

/// Decode an audio file to mono f32 PCM at 44100 Hz.
pub fn decode_audio(path: &Path) -> Result<AudioData, String> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open {}: {e}", path.display()))?;

    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| format!("Failed to probe audio format: {e}"))?;

    let mut format = probed.format;

    let track = format
        .default_track()
        .ok_or("No audio tracks found")?;

    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let source_sample_rate = codec_params.sample_rate.unwrap_or(TARGET_SAMPLE_RATE);
    let channels = codec_params.channels.map(|c| c.count()).unwrap_or(1);

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| format!("Failed to create decoder: {e}"))?;

    let mut all_samples: Vec<f32> = Vec::new();

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(symphonia::core::errors::Error::IoError(ref e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(e) => return Err(format!("Error reading packet: {e}")),
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(format!("Decode error: {e}")),
        };

        append_mono_samples(&decoded, channels, &mut all_samples);
    }

    // Resample to 44100 Hz if needed
    if source_sample_rate != TARGET_SAMPLE_RATE {
        all_samples = resample(&all_samples, source_sample_rate, TARGET_SAMPLE_RATE);
    }

    Ok(AudioData {
        samples: all_samples,
        sample_rate: TARGET_SAMPLE_RATE,
    })
}

/// Extract samples from a decoded audio buffer, mixing to mono.
fn append_mono_samples(buf: &AudioBufferRef, channels: usize, out: &mut Vec<f32>) {
    match buf {
        AudioBufferRef::F32(b) => {
            let frames = b.frames();
            for frame in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    sum += b.chan(ch)[frame];
                }
                out.push(sum / channels as f32);
            }
        }
        AudioBufferRef::S16(b) => {
            let frames = b.frames();
            for frame in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    sum += b.chan(ch)[frame] as f32 / 32768.0;
                }
                out.push(sum / channels as f32);
            }
        }
        AudioBufferRef::S32(b) => {
            let frames = b.frames();
            for frame in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    sum += b.chan(ch)[frame] as f32 / 2_147_483_648.0;
                }
                out.push(sum / channels as f32);
            }
        }
        AudioBufferRef::U8(b) => {
            let frames = b.frames();
            for frame in 0..frames {
                let mut sum = 0.0f32;
                for ch in 0..channels {
                    sum += (b.chan(ch)[frame] as f32 - 128.0) / 128.0;
                }
                out.push(sum / channels as f32);
            }
        }
        _ => {
            // Fallback: skip unsupported formats
            eprintln!("Warning: unsupported sample format, skipping packet");
        }
    }
}

/// Simple linear interpolation resampler.
fn resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = from_rate as f64 / to_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos as usize;
        let frac = (src_pos - idx as f64) as f32;

        let s0 = samples[idx];
        let s1 = if idx + 1 < samples.len() {
            samples[idx + 1]
        } else {
            s0
        };

        out.push(s0 + frac * (s1 - s0));
    }

    out
}
