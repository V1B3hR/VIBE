use std::path::Path;
use std::fs::File;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::errors::Error;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Decodes an audio file into a Vec of f32 samples (mono summed).
pub fn decode_file_to_vec<P: AsRef<Path>>(path: P) -> Result<Vec<f32>, String> {
    let src = File::open(path.as_ref()).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(src), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.as_ref().extension() {
        if let Some(ext_str) = ext.to_str() {
            hint.with_extension(ext_str);
        }
    }

    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            mss,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|e| e.to_string())?;

    let mut format = probed.format;
    let track = format.default_track().ok_or("No track found")?;
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    let mut samples: Vec<f32> = Vec::new();
    let mut channels = 0;

    while let Ok(packet) = format.next_packet() {
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(audio_buf) => {
                if channels == 0 {
                    channels = audio_buf.spec().channels.count();
                }

                use symphonia::core::audio::SampleBuffer;
                let mut sample_buf =
                    SampleBuffer::<f32>::new(audio_buf.capacity() as u64, *audio_buf.spec());
                sample_buf.copy_interleaved_ref(audio_buf);
                
                let chunk = sample_buf.samples();
                if channels > 1 {
                    // Sum to mono for analysis
                    for frame in chunk.chunks_exact(channels) {
                        samples.push(frame.iter().sum::<f32>() / channels as f32);
                    }
                } else {
                    samples.extend_from_slice(chunk);
                }
            }
            Err(Error::IoError(_)) => break,
            Err(Error::DecodeError(_)) => continue,
            Err(_) => continue,
        }
    }

    Ok(samples)
}
