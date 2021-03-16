#![feature(available_concurrency)]
#![feature(box_syntax)]
#![feature(pub_macro_rules)]
#![feature(slice_as_chunks)]
#![feature(try_trait)]

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::Path;

use clap::clap_app;

use crate::audio::AudioStretchError;
use crate::beatmap::{Beatmap, ParseError};

mod audio;
mod beatmap;
mod util;

fn main() {
    let matches = clap_app!(osurate =>
        (version: "0.1.1")
        (author: "LunarCoffee <lunarcoffee.pjc@gmail.com>")
        (about: "rate generator for osu! beatmaps")
        (@arg inputs: * #{1, u64::MAX} "Sets the input .osu file(s)")
        (@arg rates: * -r #{1, u64::MAX} "Sets the rate(s) to generate")
    ).get_matches();

    let rate_matches = matches.values_of("rates").unwrap();
    let map_paths = matches.values_of("inputs").unwrap();
    let rates = rate_matches.map(|r| r.parse::<f64>()).collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|_| util::log_fatal("invalid rate(s) specified"));
    rates.iter().any(|&r| r < 0.01).then(|| util::log_fatal("negative rates are not supported"));

    util::log_info("starting");
    map_paths.map(|p| Path::new(p)).for_each(|p| generate_rates(p, &rates));
    util::log_info("done");
}

// Generates and saves the rates in `rates` for the .osu file at `path`.
fn generate_rates(path: &Path, rates: &[f64]) {
    let path = path.canonicalize().unwrap_or_else(|_| util::log_fatal("couldn't find file"));
    let map_file = File::open(&path).unwrap_or_else(|_| util::log_fatal("couldn't open file"));
    let reader = BufReader::new(map_file);

    let map = Beatmap::parse(reader).unwrap_or_else(|e| util::log_fatal(match e {
        ParseError::UnsupportedVersion => "unsupported beatmap file format version",
        ParseError::InvalidBeatmap => "couldn't parse beatmap file",
        _ => "beatmap file i/o error",
    }));

    for rate in rates {
        // Since the map is mutated by `change_rate`, inaccuracies may accumulate when reverting a rate change. To work
        // around this, the beatmap is cloned for each rate.
        generate_rate(map.clone(), *rate, &path);
    }
}

// Generates and saves the given rate for the given beatmap.
fn generate_rate(mut map: Beatmap, rate: f64, path: &Path) {
    let parent_dir = path.parent().unwrap_or(Path::new("./"));
    let base_map_name = path.file_stem().unwrap_or_else(|| util::log_fatal("couldn't find file")).to_string_lossy();
    let old_diff_name = map.metadata.diff_name.to_string();

    map.change_rate(rate).then(|| {}).unwrap_or_else(|| util::log_fatal("invalid beatmap file"));
    audio::stretch_beatmap_audio(&mut map, parent_dir, rate).unwrap_or_else(|e| util::log_fatal(match e {
        AudioStretchError::SourceNotFound => "couldn't find mp3 file",
        AudioStretchError::InvalidSource => "couldn't parse mp3 file",
        AudioStretchError::UnsupportedChannelCount => "unsupported mp3 channel count",
        AudioStretchError::LameInitializationError => "couldn't initialize lame (is it installed?)",
        AudioStretchError::LameEncodingError => "lame mp3 encoding error",
        _ => "mp3 output i/o error",
    }));

    // Generate a new file name with the rate in the beatmap difficulty area.
    let gen_path = parent_dir.join(path.to_string_lossy().replace(&old_diff_name, &*map.metadata.diff_name));
    let mut gen_file = File::create(gen_path)
        .unwrap_or_else(|_| util::log_fatal("couldn't create new beatmap rate file"));
    gen_file.write_all(map.into_string().as_bytes())
        .unwrap_or_else(|_| util::log_fatal("couldn't write new beatmap rate file"));

    util::log_info(format!("generated {}x rate of {}", rate, base_map_name));
}
