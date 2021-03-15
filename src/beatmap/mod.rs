use std::fmt;
use std::fmt::Formatter;
use std::io::BufRead;

use crate::beatmap::parser::Parser;

mod parser;

#[derive(Debug)]
pub struct Beatmap {
    general_info: GeneralInfo,
    editor_info: EditorInfo,
    metadata: Metadata,
    difficulty_info: DifficultyInfo,
    events: Events,
    timing_points: TimingPoints,
    hit_objects: HitObjects,
}

impl Beatmap {
    pub fn parse(reader: impl BufRead) -> parser::Result<Beatmap> {
        Parser::new(reader).parse()
    }
}

#[derive(Debug)]
pub struct GeneralInfo {
    audio_file: String,
    audio_lead_in: i32,
    preview_time: i32,
    // countdown: i32,
    // sample_set: String,
    // stack_leniency: f64,
    // mode: i32,
    // letterbox_break: bool,
    // use_skin_sprites: bool,
    // overlay_pos: String,
    // skin_pref: String,
    // epilepsy_warning: bool,
    // countdown_offset: i32,
    // special_style: bool,
    // widescreen_sb: bool,
    // scale_sample_rate: bool,
    rest: String,
}

// pub struct EditorInfo {
//     bookmarks: String,
//     distance_spacing: f64,
//     beat_divisor: f64,
//     grid_size: i32,
//     timeline_zoom: f64,
// }

#[derive(Debug)]
pub struct EditorInfo(String);

#[derive(Debug)]
pub struct Metadata {
    // title: String,
    // raw_title: String,
    // artist: String,
    // raw_artist: String,
    // creator: String,
    diff_name: String,
    // source: String,
    // tags: String,
    // map_id: i32,
    // mapset_id: i32,
    rest: String,
}

// pub struct DifficultyInfo {
//     hp: f64,
//     cs_or_keys: f64,
//     od: f64,
//     ar: f64,
//     slider_multiplier: f64,
//     slider_tick_rate: f64,
// }

#[derive(Debug)]
pub struct DifficultyInfo(String);

#[derive(Debug)]
pub struct Events(String);

#[derive(Debug)]
pub struct TimingPoints(Vec<TimingPoint>);

#[derive(Debug)]
pub struct TimingPoint {
    time: i32,
    beat_length: f64,
    meter: i32,
    sample_set: i32,
    sample_index: i32,
    volume: i32,
    uninherited: bool,
    effects: i32,
}

#[derive(Debug)]
pub struct Colors(String);

#[derive(Debug)]
pub struct HitObjects(Vec<HitObject>);

#[derive(Debug)]
pub struct HitObject {
    x: i32,
    y: i32,
    time: i32,
    kind: i32,
    hit_sound: i32,
    params: HitObjectParams,
    hit_sample: String,
}

#[derive(Debug)]
pub enum HitObjectParams {
    HitCircle,
    // None of the slider object parameters are useful. TODO yes?
    Slider(String),
    Spinner { end_time: i32 },
    LongNote { end_time: i32 },
}
