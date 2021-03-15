use std::io::BufRead;

use crate::beatmap::parser::Parser;

mod parser;

#[derive(Debug)]
pub struct Beatmap {
    general_info: GeneralInfo,
    editor_info: EditorInfo,
    metadata: Metadata,
    difficulty: DifficultyInfo,
    events: Events,
    timing_points: Vec<TimingPoint>,
    colors: Option<Colors>,
    hit_objects: Vec<HitObject>,
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
    rest: String,
}

#[derive(Debug)]
pub struct EditorInfo(String);

#[derive(Debug)]
pub struct Metadata {
    diff_name: String,
    rest: String,
}

#[derive(Debug)]
pub struct DifficultyInfo(String);

#[derive(Debug)]
pub struct Events(String);

#[derive(Debug)]
pub struct TimingPoint {
    time: i32,
    beat_len: f64,
    rest: String,
}

#[derive(Debug)]
pub struct Colors(String);

#[derive(Debug)]
pub struct HitObject {
    time: i32,
    params: HitObjectParams,
    rest_parts: Vec<String>, // ["x,y", "type,hit_sound", "unused_object_params,hit_sample"]
}

#[derive(Debug)]
pub enum HitObjectParams {
    NoneUseful,
    Spinner(i32),
    Slider(i32),
}
