use std::{result, thread};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use dasp::{signal, Signal};
use dasp::interpolate::linear::Linear;
use lame::Lame;
use minimp3::Decoder;

use crate::beatmap::Beatmap;
use crate::util;

#[derive(Debug)]
pub enum AudioStretchError {
    SourceNotFound,
    InvalidSource,
    UnsupportedChannelCount,
    LameInitializationError,
    LameEncodingError,
    DestinationIoError,
}

impl From<lame::Error> for AudioStretchError {
    fn from(_: lame::Error) -> Self {
        Self::LameInitializationError
    }
}

type Result<T> = result::Result<T, AudioStretchError>;

// Stretches the audio associated with the given `map` by a factor of `rate`, updating metadata.
pub fn stretch_beatmap_audio(map: &mut Beatmap, dir: &Path, rate: f64) -> Result<()> {
    let old_path = dir.join(&map.general_info.audio_file);
    let old_audio = File::open(&old_path).or(Err(AudioStretchError::SourceNotFound))?;

    // This looks like "audio.mp3" -> "audio_1_2.mp3" for a rate of 1.2.
    let new_path = dir.join(format!(
        "{}_{}.{}",
        old_path.file_stem().ok_or(AudioStretchError::InvalidSource)?.to_string_lossy(),
        rate.to_string().replace('.', "_"),
        old_path.extension().ok_or(AudioStretchError::InvalidSource)?.to_string_lossy(),
    ));
    let mut new_audio = File::create(&new_path).or(Err(AudioStretchError::DestinationIoError))?;
    stretch(old_audio, &mut new_audio, rate)?;

    // This should be fine, since the file name was created just above.
    map.general_info.audio_file = new_path.file_name().unwrap().to_str().unwrap().to_string();
    Ok(())
}

// Stretches MP3 audio read from `src` by a factor of `rate`, writing the output to `dest` as MP3 audio.
fn stretch(src: impl Read, dest: &mut impl Write, rate: f64) -> Result<()> {
    // Decode source MP3 data into i16 PCM data.
    let mut decoder = Decoder::new(src);
    let mut frames = vec![];
    while let Ok(frame) = decoder.next_frame() {
        frames.push(frame);
    }
    match decoder.next_frame() {
        Err(minimp3::Error::Eof) | Err(minimp3::Error::SkippedData) => {}
        _ => return Err(AudioStretchError::InvalidSource),
    }

    let channels = frames[0].channels;
    util::verify(channels <= 2, AudioStretchError::UnsupportedChannelCount)?;
    let sample_rate = frames[0].sample_rate;
    let bitrate = frames[0].bitrate;

    // Gather samples from each frame and resample.
    let samples = frames.into_iter().flat_map(|f| f.data).collect();
    let concurrency = thread::available_concurrency().map(|n| n.get()).unwrap_or(2);
    let (samples_l, samples_r) = resample_parallel(samples, rate, concurrency);

    let mut lame = Lame::new().ok_or(AudioStretchError::LameInitializationError)?;
    lame.init_params()?;
    lame.set_sample_rate(sample_rate as u32)?;
    lame.set_quality(9)?;
    lame.set_kilobitrate(bitrate.min(128))?;

    // Encode the stretched PCM data to MP3, writing it to `dest`.
    let mut buf = vec![0; samples_l.len()];
    let written = lame.encode(&samples_l, &samples_r, &mut buf).or(Err(AudioStretchError::LameEncodingError))?;
    dest.write_all(&buf[..written]).or(Err(AudioStretchError::DestinationIoError))
}

// Resamples dual channel PCM `samples` by a factor of `rate` in parallel with `threads` worker threads.
fn resample_parallel(samples: Vec<i16>, rate: f64, n_threads: usize) -> (Vec<i16>, Vec<i16>) {
    // Split the samples into equally sized chunks and spawn a thread to process each.
    let n_chunks = (samples.len() as f64 / n_threads as f64).ceil() as usize;
    let chunks = samples.chunks(n_chunks).map(|c| c.to_vec());
    let handles = chunks.map(|c| thread::spawn(move || resample_chunk(c, rate))).collect::<Vec<_>>();

    // Recombine the resampled chunks.
    handles.into_iter().map(|h| h.join().unwrap()).flatten().unzip()
}

// Helper function to resample a chunk of PCM samples.
fn resample_chunk(samples: Vec<i16>, rate: f64) -> Vec<(i16, i16)> {
    let mut src = signal::from_interleaved_samples_iter::<_, [i16; 2]>(samples);
    let lerp = Linear::new(src.next(), src.next());
    src.scale_hz(lerp, rate).until_exhausted().map(|[l, r]| (l, r)).collect()
}
