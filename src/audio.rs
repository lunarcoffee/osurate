use std::{panic, result, thread};
use std::io::{Read, Write};

use lame::Lame;
use minimp3::Decoder;
use rubato::{InterpolationParameters, InterpolationType, Resampler, SincFixedIn, WindowFunction};

#[derive(Clone, Copy, Debug)]
pub enum AudioStretchError {
    InvalidSource,
    UnsupportedChannelCount,
    ResampleError,
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

// Stretches MP3 audio read from `src` by a factor of `rate`, writing the output to `dest` as MP3 audio.
pub fn stretch(src: impl Read, dest: &mut impl Write, rate: f64) -> Result<()> {
    // Decode source MP3 data into raw PCM data.
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
    (channels <= 2).then(|| {}).ok_or(AudioStretchError::UnsupportedChannelCount)?;
    let sample_rate = frames[0].sample_rate;
    let bitrate = frames[0].bitrate;

    // Gather samples and convert them to f32 PCM.
    let i16_to_f32 = |n| n as f32 / i16::MAX as f32;
    let samples = frames.into_iter().flat_map(|f| f.data.into_iter().map(i16_to_f32)).collect::<Vec<_>>();

    // Turn samples into left and right channels and resample them.
    let samples_lr: (Vec<_>, Vec<_>) = match channels {
        1 => (samples.clone(), samples),
        _ => samples.as_chunks::<2>().0.iter().map(|&[l, r]| (l, r)).unzip(),
    };
    let concurrency = thread::available_concurrency().map(|n| n.get()).unwrap_or(2);
    let (samples_l, samples_r) = resample_parallel(samples_lr, rate, concurrency)?;

    let mut lame = Lame::new().ok_or(AudioStretchError::LameInitializationError)?;
    lame.init_params()?;
    lame.set_sample_rate(sample_rate as u32)?;
    lame.set_quality(6)?;
    lame.set_kilobitrate(bitrate)?;

    // Encode the stretched PCM data to MP3, writing it to `dest`.
    let mut buf = vec![0; samples_l.len()];
    let written = lame.encode(&samples_l, &samples_r, &mut buf).or(Err(AudioStretchError::LameEncodingError))?;
    dest.write_all(&buf[..written]).or(Err(AudioStretchError::DestinationIoError))
}

// Resamples dual channel f32 PCM `samples` by a factor of `rate` in parallel with `threads` worker threads, returning
// the new samples as i16 PCM data.
fn resample_parallel(samples: (Vec<f32>, Vec<f32>), rate: f64, n_threads: usize) -> Result<(Vec<i16>, Vec<i16>)> {
    // Split the samples into equally sized chunks based on the number of worker threads.
    let sample_pairs = samples.0.into_iter().zip(samples.1).collect::<Vec<_>>();
    let sample_chunks = sample_pairs.chunks(sample_pairs.len() / n_threads);

    let mut handles = vec![];
    for sample_chunk in sample_chunks {
        // Spawn a thread to process each chunk.
        let (chunk_l, chunk_r) = sample_chunk.iter().map(|p| *p).unzip();
        handles.push(thread::spawn(move || resample_f32_to_i16([chunk_l, chunk_r], rate)));
    }

    // Recombine the resampled chunks.
    let resampled_chunks = handles.into_iter().map(|h| h.join().unwrap()).collect::<Result<Vec<_>>>()?;
    Ok(resampled_chunks.into_iter().flatten().unzip())
}

// Helper function to resample a chunk of PCM samples.
fn resample_f32_to_i16(samples: [Vec<f32>; 2], rate: f64) -> Result<Vec<(i16, i16)>> {
    // These are optimized heavily for speed over quality.
    let params = InterpolationParameters {
        sinc_len: 64,
        f_cutoff: 0.95,
        interpolation: InterpolationType::Nearest,
        oversampling_factor: 64,
        window: WindowFunction::Hann,
    };

    // Temporarily silence panics; the resampling panics from a failed assertion when `rate` is too high (> ~1.5).
    let prev_hook = panic::take_hook();
    panic::set_hook(box |_| {});

    let resampled = panic::catch_unwind(|| {
        let mut resampler = SincFixedIn::<f32>::new(1.0 / rate, params, samples[0].len(), 2);
        resampler.process(&samples).or(Err(AudioStretchError::ResampleError))
    }).map_err(|_| AudioStretchError::ResampleError)??;

    panic::set_hook(prev_hook);

    // Convert resampled data to i16 PCM.
    let f32_to_i16 = |n| (n * i16::MAX as f32) as i16;
    Ok(resampled[0].iter().zip(&resampled[1]).map(|(l, r)| (f32_to_i16(l), f32_to_i16(r))).collect())
}
