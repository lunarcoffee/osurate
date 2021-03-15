use std::{io, result};
use std::error::Error;
use std::io::{BufRead, Read};
use std::str::FromStr;

use crate::beatmap::{Beatmap, DifficultyInfo, EditorInfo, Events, GeneralInfo, Metadata, TimingPoint, TimingPoints};
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

pub type Result<T> = result::Result<T, ParseError>;

pub struct Parser<R: BufRead> {
    reader: R,
}

impl<R: BufRead> Parser<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    pub fn parse(&mut self) -> Result<Beatmap> {
        let header = self.read_line()?;
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
        let difficulty_info = DifficultyInfo(rest);

        verify_ff(next_section_header == "[Events]")?;
        let (rest, next_section_header) = self.read_section()?;
        let events = Events(rest);

        verify_ff(next_section_header == "[TimingPoints]")?;


        println!("{:?}\n{:?}\n{:?}\n{:?}\n{:?}", general_info, editor_info, metadata, difficulty_info, events);

        Err(ParseError::IoError)
    }

    // Parse the necessary parts of the general info section.
    fn parse_general_info(&mut self) -> Result<(GeneralInfo, String)> {
        let mut audio_file = String::new();
        let mut audio_lead_in = 0;
        let mut preview_time = -1;
        let mut rest = String::new();

        let mut line = self.read_line()?;
        while !is_section_header_or_eof(&line) {
            let (key, value) = line.split_once(":").ok_or(ParseError::InvalidBeatmap)?;
            let value = value[1..].to_string(); // Trim off mandatory space.
            match key {
                "AudioFilename" => audio_file = value,
                "AudioLeadIn" => audio_lead_in = value.parse().or(Err(ParseError::InvalidBeatmap))?,
                "PreviewTime" => preview_time = value.parse().or(Err(ParseError::InvalidBeatmap))?,
                _ => rest += &(line + "\n"),
            }
            line = self.read_line()?;
        }

        // Verify that required values were parsed.
        verify_ff(!audio_file.is_empty())?;
        Ok((GeneralInfo { audio_file, audio_lead_in, preview_time, rest }, line))
    }

    // Parse the necessary parts of the metadata section.
    fn parse_metadata(&mut self) -> Result<(Metadata, String)> {
        let mut diff_name = String::new();
        let mut rest = String::new();

        let mut line = self.read_line()?;
        while !is_section_header_or_eof(&line) {
            let (key, value) = line.split_once(":").ok_or(ParseError::InvalidBeatmap)?;
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

    fn parse_timing_points(self) -> Result<(TimingPoints, String)> {
        Ok((TimingPoints(vec![]), "".to_string()))
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

        // Skip empty and comments.
        if &buf[..] == "\n" || &buf[..] == "\r\n" || buf.starts_with("//") {
            self.read_line()
        } else {
            Ok(buf.trim_end().to_string())
        }
    }
}

// Convenient wrapper over `util::verify` specifically for verifying parts of the beatmap file format.
fn verify_ff(cond: bool) -> Result<()> {
    verify(cond, ParseError::InvalidBeatmap)
}

// Checks if `line` is a section header (i.e. "[Metadata]") or was the result of reaching EOF.
fn is_section_header_or_eof(line: &str) -> bool {
    line.chars().next() == Some('[') && line.chars().last() == Some(']') || line.is_empty()
}
