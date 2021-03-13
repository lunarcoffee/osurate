use std::io::{Read, Write};
use std::panic;

use lame::Lame;
use minimp3::{Decoder, Error};
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

// Stretches MP3 audio read from `src` by a factor of `rate`, writing the output to `dest` as MP3 audio.
pub fn stretch(src: impl Read, dest: &mut impl Write, rate: f64) -> Result<(), AudioStretchError> {
    // Decode source MP3 data into raw PCM data.
    let mut decoder = Decoder::new(src);
    let mut frames = vec![];
    while let Ok(frame) = decoder.next_frame() {
        frames.push(frame);
    }
    match decoder.next_frame() {
        Err(Error::Eof) | Err(Error::SkippedData) => {}
        _ => return Err(AudioStretchError::InvalidSource),
    }

    let channels = frames[0].channels;
    let sample_rate = frames[0].sample_rate;
    let bitrate = frames[0].bitrate;

    // This LAME binding only supports 2 channels.
    if channels > 2 {
        return Err(AudioStretchError::UnsupportedChannelCount);
    }

    // Gather samples and convert them to f32 PCM.
    let i16_to_f32 = |n| n as f32 / i16::MAX as f32;
    let samples = frames.into_iter().flat_map(|f| f.data.into_iter().map(i16_to_f32)).collect::<Vec<_>>();

    // Turn samples into left and right channels and resample them.
    let samples_lr: (Vec<_>, Vec<_>) = match channels {
        1 => (samples.clone(), samples),
        _ => samples.as_chunks::<2>().0.iter().map(|&[l, r]| (l, r)).unzip(),
    };
    let (samples_l, samples_r) = resample_f32_to_i16([samples_lr.0, samples_lr.1], rate)?;

    let lame_err = AudioStretchError::LameInitializationError;
    let mut lame = Lame::new().ok_or(lame_err)?;
    lame.init_params().or(Err(lame_err))?;
    lame.set_sample_rate(sample_rate as u32).or(Err(lame_err))?;
    lame.set_quality(5).or(Err(lame_err))?;
    lame.set_kilobitrate(bitrate).or(Err(lame_err))?;

    // Encode the stretched PCM data to MP3, writing it to `dest`.
    let mut buf = vec![0; samples_l.len()];
    let written = lame.encode(&samples_l, &samples_r, &mut buf).or(Err(AudioStretchError::LameEncodingError))?;
    dest.write_all(&buf[..written]).or(Err(AudioStretchError::DestinationIoError))
}

// Resamples dual channel f32 PCM `samples` by a factor of `rate`, returning them as i16 PCM data.
fn resample_f32_to_i16(samples: [Vec<f32>; 2], rate: f64) -> Result<(Vec<i16>, Vec<i16>), AudioStretchError> {
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

    let resampled_frames = panic::catch_unwind(|| {
        let mut resampler = SincFixedIn::<f32>::new(1.0 / rate, params, samples[0].len(), 2);
        resampler.process(&samples).or(Err(AudioStretchError::ResampleError))
    }).map_err(|_| AudioStretchError::ResampleError)??;

    panic::set_hook(prev_hook);

    let f32_to_i16 = |n| (n * i16::MAX as f32) as i16;
    let frames_l = resampled_frames[0].iter().map(f32_to_i16).collect();
    let frames_r = resampled_frames[1].iter().map(f32_to_i16).collect();
    Ok((frames_l, frames_r))
}
