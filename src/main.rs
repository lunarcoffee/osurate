#![feature(available_concurrency)]
#![feature(box_syntax)]
#![feature(pub_macro_rules)]
#![feature(slice_as_chunks)]

use std::fs::File;
use std::io::{BufReader, Cursor, Read};

use crate::beatmap::Beatmap;

mod audio;
mod beatmap;
mod util;

fn main() {
    // let mut src = vec![];
    // File::open("resources/audio.mp3").unwrap().read_to_end(&mut src);
    // let src = Cursor::new(src);
    // let mut dest = File::create("resources/out.mp3").unwrap();
    // println!("{:?}", audio::stretch(src, &mut dest, 1.2));

    let map = File::open("resources/enigma.osu").unwrap();
    let reader = BufReader::new(map);
    println!("{:?}", Beatmap::parse(reader));
}
