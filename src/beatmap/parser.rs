use std::{io, result};
use std::io::BufRead;
use std::option::NoneError;
use std::str::FromStr;

use crate::beatmap::{
    Beatmap, Colors, DifficultyInfo, EditorInfo, Events, GeneralInfo, HitObject, HitObjectParams, Metadata,
    TimingPoint,
};
use crate::util;
use crate::util::verify;

#[derive(Debug)]
pub enum ParseError {
    UnsupportedVersion,
    InvalidBeatmap,
    IoError,
}

impl From<io::Error> for ParseError {
    fn from(_: io::Error) -> Self {
        ParseError::IoError
    }
}

impl From<NoneError> for ParseError {
    fn from(_: NoneError) -> Self {
        ParseError::InvalidBeatmap
    }
}

pub type Result<T> = result::Result<T, ParseError>;

pub struct Parser<R: BufRead> {
    reader: R,
}

impl<R: BufRead> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn parse(&mut self) -> Result<Beatmap> {
        let header = trim_utf8_bom(self.read_line()?)?;
        verify_ff(header.starts_with("osu file format v"))?;
        util::verify(&header[17..] == "14", ParseError::UnsupportedVersion)?;

        verify_ff(self.read_line()? == "[General]")?;
        let (general_info, next_section_header) = self.parse_general_info()?;

        verify_ff(next_section_header == "[Editor]")?;
        let (rest, next_section_header) = self.read_section()?;
        let editor_info = EditorInfo(rest);

        verify_ff(next_section_header == "[Metadata]")?;
        let (metadata, next_section_header) = self.parse_metadata()?;

        verify_ff(next_section_header == "[Difficulty]")?;
        let (rest, next_section_header) = self.read_section()?;
        let difficulty = DifficultyInfo(rest);

        verify_ff(next_section_header == "[Events]")?;
        let (rest, next_section_header) = self.read_section()?;
        let events = Events(rest);

        verify_ff(next_section_header == "[TimingPoints]")?;
        let (timing_points, mut next_section_header) = self.parse_timing_points()?;

        // This section appears to be optional.
        let colors = if next_section_header == "[Colours]" {
            let (rest, next) = self.read_section()?;
            next_section_header = next;
            Some(Colors(rest))
        } else {
            None
        };

        verify_ff(next_section_header == "[HitObjects]")?;
        let hit_objects = self.parse_hit_objects()?;

        Ok(Beatmap { general_info, editor_info, metadata, difficulty, events, timing_points, colors, hit_objects })
    }

    fn parse_general_info(&mut self) -> Result<(GeneralInfo, String)> {
        let mut audio_file = String::new();
        let mut preview_time = -1;
        let mut rest = String::new();

        let mut line = self.read_line()?;
        while !is_section_header_or_eof(&line) {
            let (key, value) = line.split_once(": ")?;
            match key {
                "AudioFilename" => audio_file = value.to_string(),
                "PreviewTime" => preview_time = parse_ff(value)?,
                _ => rest += &(line + "\n"),
            }
            line = self.read_line()?;
        }

        // Verify that required values were parsed.
        verify_ff(!audio_file.is_empty())?;
        Ok((GeneralInfo { audio_file, preview_time, rest }, line))
    }

    fn parse_metadata(&mut self) -> Result<(Metadata, String)> {
        let mut diff_name = String::new();
        let mut rest = String::new();

        let mut line = self.read_line()?;
        while !is_section_header_or_eof(&line) {
            let (key, value) = line.split_once(":")?;
            match key {
                "Version" => diff_name = value.to_string(),
                _ => rest += &(line + "\n"),
            }
            line = self.read_line()?;
        }

        // Verify that required values were parsed.
        verify_ff(!diff_name.is_empty())?;
        Ok((Metadata { diff_name, rest }, line))
    }

    fn parse_timing_points(&mut self) -> Result<(Vec<TimingPoint>, String)> {
        let mut timing_points = vec![];

        let mut line = self.read_line()?;
        while !is_section_header_or_eof(&line) {
            let split = line.splitn(3, ',').collect::<Vec<_>>();
            verify_ff(split.len() == 3)?;

            let time = parse_ff(split[0])?;
            let beat_len = parse_ff(split[1])?;
            timing_points.push(TimingPoint { time, beat_len, rest: split[2].to_string() });
            line = self.read_line()?;
        }
        Ok((timing_points, line))
    }

    fn parse_hit_objects(&mut self) -> Result<Vec<HitObject>> {
        let mut hit_objects = vec![];

        let mut line = self.read_line()?;
        while !is_section_header_or_eof(&line) {
            let mut split = line.split(',');
            let mut rest_parts = vec![]; // See `beatmap/mod.rs`.

            rest_parts.push(format!("{},{}", split.next()?, split.next()?));
            let time = parse_ff(split.next()?)?;
            let kind = parse_ff::<i32>(split.next()?)?;
            rest_parts.push(format!("{},{}", kind, split.next()?));

            let params = if kind & (1 << 0) == 1 || kind & (1 << 1) == 2 {
                HitObjectParams::NoneUseful
            } else if kind & (1 << 3) == 8 {
                HitObjectParams::Spinner(parse_ff(split.next()?)?)
            } else if kind & (1 << 7) == 128 {
                let end_time = split.clone().next()?.split_once(':')?.0;
                HitObjectParams::LongNote(parse_ff(end_time)?)
            } else {
                return Err(ParseError::InvalidBeatmap);
            };
            rest_parts.push(split.collect::<Vec<_>>().join(","));

            hit_objects.push(HitObject { time, params, rest_parts });
            line = self.read_line()?;
        }

        // Verify that EOF has been reached.
        verify_ff(line.is_empty())?;
        Ok(hit_objects)
    }

    // Read an entire section to a string without any special parsing.
    fn read_section(&mut self) -> io::Result<(String, String)> {
        let mut rest = String::new();

        let mut line = self.read_line()?;
        while !is_section_header_or_eof(&line) {
            rest += &(line + "\n");
            line = self.read_line()?;
        };
        Ok((rest, line))
    }

    // Reads a line from `reader`, discarding the newline delimiter and skipping empty lines and comments.
    fn read_line(&mut self) -> io::Result<String> {
        let mut buf = String::new();

        // Return an empty string on EOF.
        if self.reader.read_line(&mut buf)? == 0 {
            return Ok(buf);
        }

        // Skip empty lines and comments.
        if buf.trim().is_empty() || buf.starts_with("//") {
            self.read_line()
        } else {
            Ok(buf.trim_end().to_string())
        }
    }
}

// Convenience wrapper over `util::verify` specifically for verifying parts of a beatmap.
fn verify_ff(cond: bool) -> Result<()> {
    verify(cond, ParseError::InvalidBeatmap)
}

// Convenience wrapper over `parse` specifically for parsing required values in a beatmap.
fn parse_ff<F: FromStr>(str: &str) -> Result<F> {
    str.parse().or(Err(ParseError::InvalidBeatmap))
}

// Checks if `line` is a section header (i.e. "[Metadata]") or was the result of reaching EOF.
fn is_section_header_or_eof(line: &str) -> bool {
    line.chars().next() == Some('[') && line.chars().last() == Some(']') || line.is_empty()
}

// Trims the byte order mark from the start of a UTF-8 string, if present.
fn trim_utf8_bom(line: String) -> Option<String> {
    if line.as_bytes().starts_with(b"\xef\xbb\xbf") {
        String::from_utf8(line.as_bytes()[3..].to_vec()).ok()
    } else {
        Some(line.to_string())
    }
}
