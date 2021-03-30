#![feature(available_concurrency)]
#![feature(iter_intersperse)]
#![feature(try_trait)]

use std::fs::File;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};

use clap::clap_app;

use crate::audio::AudioStretchError;
use crate::beatmap::{Beatmap, ParseError};

mod audio;
mod beatmap;
mod gui;
mod util;

fn main() {
    // Change help text if compiled without GUI support.
    let mut gui_help = "enters gui mode".to_string();
    if !cfg!(feature = "gui") {
        gui_help += " (unavailable; recompile with `--features gui`)"
    }
    let gui_help = gui_help.as_str();
    let matches = clap_app!(osurate =>
        (version: "0.2.0")
        (author: "LunarCoffee <lunarcoffee.pjc@gmail.com>")
        (about: "rate generator for osu! beatmaps")
        (@arg gui: -g conflicts_with[inputs rates] required_unless[inputs] gui_help)
        (@arg inputs: #{1, u64::MAX} requires[rates] required_unless[gui] "sets the input .osu file(s)")
        (@arg rates: -r #{1, u64::MAX} requires[inputs] "sets the rate(s) to generate")
        (help_message: "prints help information")
        (version_message: "prints version information")
    ).get_matches();

    if matches.is_present("gui") {
        #[cfg(feature = "gui")] gui::run_gui(); // This call diverges.
        util::log_fatal("osurate was not compiled with gui support; recompile with `--features gui`");
    } else {
        let rate_matches = matches.values_of("rates").unwrap();
        let map_paths = matches.values_of("inputs").unwrap();

        let rates = rate_matches.map(|r| r.parse::<f64>()).collect::<Result<Vec<_>, _>>()
            .unwrap_or_else(|_| util::log_fatal("invalid rate(s) specified"));
        rates.iter().any(|&r| r < 0.01).then(|| util::log_fatal("rates below 0.01 are not supported"));

        util::log_info("starting...");
        for path in map_paths.map(|p| Path::new(p)) {
            if let Err(e) = generate_rates(&path.to_path_buf(), &rates) {
                util::log_fatal(e);
            }
        }
    }
}

// Generates and saves the rates in `rates` for the .osu file at `path`. The returned value is the name of the map,
// used for user-facing logging.
fn generate_rates(path: &PathBuf, rates: &[f64]) -> Result<String, String> {
    let path = path.canonicalize().map_err(|_| "couldn't find file")?;
    let base_map_name = path.file_stem().ok_or_else(|| "couldn't find file").map(|s| s.to_string_lossy())?;
    let map_file = File::open(&path).map_err(|_| "couldn't open file")?;
    let reader = BufReader::new(map_file);

    let map = Beatmap::parse(reader).map_err(|e| match e {
        ParseError::UnsupportedVersion => "unsupported beatmap file format version",
        ParseError::InvalidBeatmap => "couldn't parse beatmap file",
        _ => "beatmap file i/o error",
    })?;

    for rate in rates {
        // Since the map is mutated by `change_rate`, inaccuracies may accumulate when reverting a rate change. To work
        // around this, the beatmap is cloned for each rate.
        generate_rate(map.clone(), *rate, &path)?;
        util::log_info(format!("generated {}x rate of {}", rate, base_map_name));
    }
    Ok(base_map_name.to_string())
}

// Generates and saves the given rate for the given beatmap.
fn generate_rate(mut map: Beatmap, rate: f64, path: &PathBuf) -> Result<(), String> {
    let parent_dir = path.parent().unwrap_or(Path::new("./"));
    let old_diff_name = map.metadata.diff_name.to_string();

    map.change_rate(rate).then(|| {}).ok_or_else(|| "invalid beatmap file")?;
    audio::stretch_beatmap_audio(&mut map, parent_dir, rate).map_err(|e| match e {
        AudioStretchError::SourceNotFound => "couldn't find mp3 file",
        AudioStretchError::InvalidSource => "couldn't parse mp3 file",
        AudioStretchError::UnsupportedChannelCount => "unsupported mp3 channel count",
        AudioStretchError::LameInitializationError => "couldn't initialize lame (is it installed?)",
        AudioStretchError::LameEncodingError => "lame mp3 encoding error",
        _ => "mp3 output i/o error",
    })?;

    // Generate a new file name with the rate in the beatmap difficulty area.
    let gen_path = parent_dir.join(path.to_string_lossy().replace(&old_diff_name, &*map.metadata.diff_name));
    let mut gen_file = File::create(gen_path).map_err(|_| "couldn't create new beatmap rate file")?;
    gen_file.write_all(map.into_string().as_bytes()).map_err(|_| "couldn't write new beatmap rate file")?;
    Ok(())
}
