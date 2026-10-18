//! The library's part of a voice pipeline: which stretch of which
//! recording each line is, a cache key for it, the lip-sync file from the
//! mouth cues, and the voice file the game plays. Cutting the audio
//! (ffmpeg) and finding the cues (Rhubarb Lip Sync) are the caller's: the
//! library hands it [`Span`]s, and takes Rhubarb's JSON back with
//! [`take_cues`].
//!
//! A line is a stretch of a take: segments `first..=last` of the take's
//! transcript (AWS Transcribe's JSON, whose `results.audio_segments` carry
//! each segment's start and end in seconds), or the whole take. Its span
//! keeps [`LEAD`] seconds before the first word and after the last.

use std::collections::HashSet;

use rakata_lip::Lip;
use rakata_res::{ContentHash, ResRef};
use serde_json::Value as Json;
use sha2::{Digest, Sha256};

use super::lip::{from_cues, rhubarb_shape};
use super::{Error, Result};

/// Seconds kept before a span's first word and after its last.
pub const LEAD: f64 = 0.15;

/// The 58 bytes every voice file under the game's `streamwaves/` begins
/// with (all 13,799 of KotOR 1's): a RIFF header declaring mono 8-bit PCM
/// at 22,050 Hz, a `fact` chunk, and a `data` chunk of length zero, the
/// MP3 stream following it to the end of the file.
pub const WAV_HEADER: [u8; 58] = *b"RIFF\x32\x00\x00\x00WAVEfmt \x12\x00\x00\x00\x01\x00\x01\x00\x22\x56\x00\x00\x22\x56\x00\x00\x01\x00\x08\x00\x00\x00fact\x04\x00\x00\x00\x00\x00\x00\x00data\x00\x00\x00\x00";

/// One recording: its name, the hash of its audio, and its transcript's
/// segments as (start, end) seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct Take {
    pub name: String,
    pub hash: ContentHash,
    pub segments: Vec<(f64, f64)>,
}

impl Take {
    /// A take from its audio's bytes and its AWS Transcribe JSON. Fails
    /// when the JSON has no `results.audio_segments` with a start and end
    /// on each (`RE1412`).
    pub fn from_transcript(name: &str, audio: &[u8], transcript: &str) -> Result<Take> {
        let bad = |reason: &str| Error::BadCues {
            what: format!("the transcript of {name}"),
            reason: reason.to_string(),
        };
        let json: Json = serde_json::from_str(transcript).map_err(|e| bad(&e.to_string()))?;
        let segments = json
            .pointer("/results/audio_segments")
            .and_then(Json::as_array)
            .ok_or_else(|| bad("no results.audio_segments"))?
            .iter()
            .map(|s| Some((seconds(s.get("start_time")?)?, seconds(s.get("end_time")?)?)))
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| bad("a segment without a start_time and end_time in seconds"))?;
        Ok(Take {
            name: name.to_string(),
            hash: ContentHash::of(audio),
            segments,
        })
    }
}

/// A number of seconds, as Transcribe writes it (a string) or as a number.
fn seconds(v: &Json) -> Option<f64> {
    match v {
        Json::String(s) => s.parse().ok(),
        other => other.as_f64(),
    }
}

/// A voice line: the resref it is played by, the take it is cut from, its
/// subtitle (what Rhubarb is told was said), and its segments of the take,
/// inclusive from 0; `None` for `first` is the whole take, for `last` to
/// the end.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub resref: String,
    pub take: String,
    pub text: String,
    pub first: Option<usize>,
    pub last: Option<usize>,
}

/// The stretch of a take to cut for one line.
#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub resref: ResRef,
    pub take: String,
    pub text: String,
    /// Seconds into the take, the lead included, never below 0.
    pub start: f64,
    pub end: f64,
    /// The cache key: the SHA-256, in hex, of the take's hash and the span
    /// in milliseconds, so a line is cut once for as long as neither
    /// changes. Rhubarb's cues also depend on the text, so a cache of cues
    /// keys on the text as well.
    pub key: String,
}

impl Span {
    /// The start as ffmpeg's `-ss` takes it, to the millisecond.
    pub fn start_text(&self) -> String {
        format!("{:.3}", self.start)
    }

    /// The end as ffmpeg's `-to` takes it, to the millisecond.
    pub fn end_text(&self) -> String {
        format!("{:.3}", self.end)
    }
}

/// The span of every line, in order, `lead` seconds kept on each side.
/// Fails on a resref longer than sixteen bytes (`RE0101`), on two lines of
/// one resref in any case (`RE1411`), and on a line whose take is not
/// among `takes` or whose segments the take's transcript does not hold
/// (`RE1410`).
pub fn spans(lines: &[Line], takes: &[Take], lead: f64) -> Result<Vec<Span>> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(lines.len());
    for line in lines {
        let resref = ResRef::new(&line.resref)?;
        if !seen.insert(line.resref.to_ascii_lowercase()) {
            return Err(Error::DuplicateLine {
                resref: line.resref.clone(),
            });
        }
        let bad = |reason: String| Error::BadSpan {
            line: line.resref.clone(),
            reason,
        };
        let take = takes
            .iter()
            .find(|t| t.name == line.take)
            .ok_or_else(|| bad(format!("no take is named {}", line.take)))?;
        let n = take.segments.len();
        let first = line.first.unwrap_or(0);
        let last = line.last.unwrap_or(n.saturating_sub(1));
        if n == 0 || first > last || last >= n {
            return Err(bad(format!(
                "{} has segments 0..{}, not {first}..{last}",
                take.name,
                n.saturating_sub(1)
            )));
        }
        let start = (take.segments[first].0 - lead).max(0.0);
        let end = take.segments[last].1 + lead;
        let mut span = Span {
            resref,
            take: take.name.clone(),
            text: line.text.clone(),
            start,
            end,
            key: String::new(),
        };
        let mut h = Sha256::new();
        h.update(b"rakata voice span\0");
        h.update(take.hash.as_bytes());
        h.update(span.start_text().as_bytes());
        h.update(b"\0");
        h.update(span.end_text().as_bytes());
        span.key = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
        out.push(span);
    }
    Ok(out)
}

/// The lip-sync file from Rhubarb Lip Sync's JSON output (`-f json`, with
/// `--extendedShapes GHX`): each mouth cue at its start time as the game's
/// shape ([`rhubarb_shape`]), over the clip's `metadata.duration`, closing
/// on a rest. Fails when the JSON lacks the cues or the duration, or names
/// a shape Rhubarb does not have (`RE1412`).
pub fn take_cues(json: &str) -> Result<Lip> {
    let bad = |reason: String| Error::BadCues {
        what: "Rhubarb's cues".to_string(),
        reason,
    };
    let v: Json = serde_json::from_str(json).map_err(|e| bad(e.to_string()))?;
    let length = v
        .pointer("/metadata/duration")
        .and_then(Json::as_f64)
        .ok_or_else(|| bad("no metadata.duration".into()))?;
    let cues = v
        .get("mouthCues")
        .and_then(Json::as_array)
        .ok_or_else(|| bad("no mouthCues".into()))?
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let start = c.get("start").and_then(Json::as_f64);
            let shape = c
                .get("value")
                .and_then(Json::as_str)
                .and_then(|s| s.chars().next().filter(|_| s.len() == 1))
                .and_then(rhubarb_shape);
            match (start, shape) {
                (Some(t), Some(s)) => Ok((t as f32, s)),
                _ => Err(bad(format!(
                    "cue {i} is not a start time and one of Rhubarb's shapes A to H or X"
                ))),
            }
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(from_cues(&cues, length as f32))
}

/// The voice file the game plays for an MP3 stream: [`WAV_HEADER`], then
/// the stream.
pub fn streamwave(mp3: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(WAV_HEADER.len() + mp3.len());
    out.extend_from_slice(&WAV_HEADER);
    out.extend_from_slice(mp3);
    out
}
