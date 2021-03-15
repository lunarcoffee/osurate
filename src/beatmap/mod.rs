use std::io::BufRead;

pub use crate::beatmap::parser::ParseError;
use crate::beatmap::parser::Parser;

mod parser;

// Beatmap representation with only the necessary information for changing the rate of the map. Unused data is
// collectively stored in the `rest` field of a given struct (if present). Alternatively, if the entire section is
// unnecessary, the struct is a simple wrapper around the string contents of that section.
#[derive(Clone, Debug)]
pub struct Beatmap {
    pub general_info: GeneralInfo,
    pub editor_info: EditorInfo,
    pub metadata: Metadata,
    pub difficulty: DifficultyInfo,
    pub events: Events,
    pub timing_points: Vec<TimingPoint>,
    pub colors: Option<Colors>,
    pub hit_objects: Vec<HitObject>,
}

impl Beatmap {
    pub fn parse(reader: impl BufRead) -> parser::Result<Beatmap> {
        Parser::new(reader).parse()
    }

    // Changes the rate of the beatmap from 1.0 to `rate`. This does not change the audio nor the audio metadata.
    pub fn change_rate(&mut self, rate: f64) -> bool {
        let transform = |n| (n as f64 / rate) as i32;
        let transform_adj = |n| transform(n) + 90; // The stretched audio seems to have an ~90 ms delay.

        // Change relevant metadata.
        self.general_info.preview_time = transform_adj(self.general_info.preview_time);
        self.metadata.diff_name += &format!(" ({}x)", rate);

        // Adjust the first timing point to the delay.
        self.timing_points[0].time = transform_adj(self.timing_points[0].time);
        self.timing_points[0].beat_len /= rate;

        for mut point in &mut self.timing_points[1..] {
            point.time = transform(point.time);

            // Only re-time uninherited timing points.
            if point.beat_len.is_sign_positive() {
                point.beat_len /= rate;
            }
        }

        for mut object in &mut self.hit_objects {
            object.time = transform_adj(object.time);

            // Change the end times for relevant hit objects.
            match object.params {
                HitObjectParams::Spinner(end_time) =>
                    object.params = HitObjectParams::Spinner(transform_adj(end_time)),
                HitObjectParams::LongNote(end_time) => {
                    // Small hack to make up for a lack of forethought in data storage.
                    let rest = match object.rest_parts[2].split_once(':') {
                        Some((_, rest)) => rest,
                        _ => return false,
                    };
                    object.rest_parts[2] = transform_adj(end_time).to_string() + ":" + rest;
                }
                _ => {}
            }
        }
        true
    }

    // Converts the beatmap into its textual representation.
    pub fn into_string(self) -> String {
        format!(
            "osu file format v14\n\n{}\n{}\n{}\n{}\n{}\n[TimingPoints]\n{}\n\n{}\n[HitObjects]\n{}",
            self.general_info.into_string(),
            self.editor_info.into_string(),
            self.metadata.into_string(),
            self.difficulty.into_string(),
            self.events.into_string(),
            self.timing_points.into_iter().map(|p| p.into_string()).collect::<Vec<_>>().join("\n"),
            self.colors.map(|c| c.into_string()).unwrap_or(String::new()),
            self.hit_objects.into_iter().map(|p| p.into_string()).collect::<Vec<_>>().join("\n"),
        )
    }
}

#[derive(Clone, Debug)]
pub struct GeneralInfo {
    pub audio_file: String,
    pub preview_time: i32,
    rest: String,
}

impl GeneralInfo {
    fn into_string(self) -> String {
        format!("[General]\nAudioFilename: {}\nPreviewTime: {}\n{}", self.audio_file, self.preview_time, self.rest)
    }
}

#[derive(Clone, Debug)]
pub struct EditorInfo(String);

impl EditorInfo {
    fn into_string(self) -> String {
        format!("[Editor]\n{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Metadata {
    pub diff_name: String,
    rest: String,
}

impl Metadata {
    fn into_string(self) -> String {
        format!("[Metadata]\nVersion:{}\n{}", self.diff_name, self.rest)
    }
}

#[derive(Clone, Debug)]
pub struct DifficultyInfo(String);

impl DifficultyInfo {
    fn into_string(self) -> String {
        format!("[Difficulty]\n{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct Events(String);

impl Events {
    fn into_string(self) -> String {
        format!("[Events]\n{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct TimingPoint {
    pub time: i32,
    pub beat_len: f64,
    rest: String,
}

impl TimingPoint {
    fn into_string(self) -> String {
        format!("{},{},{}", self.time, self.beat_len, self.rest)
    }
}

#[derive(Clone, Debug)]
pub struct Colors(String);

impl Colors {
    fn into_string(self) -> String {
        format!("[Colours]\n{}", self.0)
    }
}

#[derive(Clone, Debug)]
pub struct HitObject {
    pub time: i32,
    pub params: HitObjectParams,
    rest_parts: Vec<String>, // ["x,y", "type,hit_sound", "unused_object_params,hit_sample"]
}

impl HitObject {
    fn into_string(self) -> String {
        format!(
            "{},{},{}{}{}",
            self.rest_parts[0],
            self.time,
            self.rest_parts[1],
            self.params.into_string(),
            self.rest_parts[2],
        )
    }
}

#[derive(Clone, Debug)]
pub enum HitObjectParams {
    NoneUseful,
    Spinner(i32),
    LongNote(i32),
}

impl HitObjectParams {
    fn into_string(self) -> String {
        match self {
            HitObjectParams::NoneUseful | HitObjectParams::LongNote(_) => ",".to_string(),
            HitObjectParams::Spinner(end_time) => format!(",{},", end_time),
        }
    }
}
