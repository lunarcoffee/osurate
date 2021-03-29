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
        // The stretched audio seems to have a ~75 ms delay.
        let transform_f64 = |n| n / rate + 75.;
        let transform = |n| transform_f64(n as f64) as i32;

        // Change relevant metadata.
        let preview = self.general_info.preview_time;
        self.general_info.preview_time = if preview >= 0 { transform(preview) } else { preview };
        self.metadata.diff_name += &format!(" ({}x)", rate);

        for mut point in &mut self.timing_points {
            point.time = transform_f64(point.time);

            // Only re-time uninherited timing points.
            if point.beat_len.is_sign_positive() {
                point.beat_len /= rate;
            }
        }

        for mut object in &mut self.hit_objects {
            object.time = transform(object.time);

            // Change the end times for relevant hit objects.
            match object.params {
                HitObjectParams::Spinner(end_time) => object.params = HitObjectParams::Spinner(transform(end_time)),
                HitObjectParams::LongNote(end_time) => {
                    // Small hack to make up for a lack of forethought in data storage.
                    let rest = match object.rest_parts[2].split_once(':') {
                        Some((_, rest)) => rest,
                        _ => return false,
                    };
                    object.rest_parts[2] = transform(end_time).to_string() + ":" + rest;
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
    // The spec on the wiki says `time` should be an integer, but some maps seem to violate that. `into_string` casts
    // this to an i32, since fractional millisecond differences are probably negligible.
    pub time: f64,
    pub beat_len: f64,
    rest: String,
}

impl TimingPoint {
    fn into_string(self) -> String {
        format!("{},{},{}", self.time as i32, self.beat_len, self.rest)
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
