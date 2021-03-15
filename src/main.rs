#![feature(available_concurrency)]
#![feature(box_syntax)]
#![feature(pub_macro_rules)]
#![feature(slice_as_chunks)]
#![feature(try_trait)]

use std::fs::File;
use std::io::{BufReader, Write};

use crate::beatmap::Beatmap;

mod audio;
mod beatmap;
mod util;

fn main() {
    let rate = 1.1;
    let map = File::open("enigma.osu").unwrap();
    let reader = BufReader::new(map);
    let mut map = Beatmap::parse(reader).unwrap();
    map.change_rate(rate);
    audio::stretch_beatmap_audio(&mut map, rate);
    let str = map.to_string();
    File::create(format!("Toromaru - Enigma (Kawawa) [{}].osu", map.metadata.diff_name)).unwrap().write_all(str.as_bytes());
}
