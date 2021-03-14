#![feature(available_concurrency)]
#![feature(box_syntax)]
#![feature(slice_as_chunks)]

use std::fs::File;

mod audio;

fn main() {
    let src = File::open("resources/audio.mp3").unwrap();
    let mut dest = File::create("resources/out.mp3").unwrap();
    println!("{:?}", audio::stretch(src, &mut dest, 1.5));
}
