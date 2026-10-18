//! A voice-over's duration from the head of its file, without decoding it.
//!
//! KotOR's `streamwaves/` files are RIFF WAVE. Nearly all of the vanilla
//! ones are MP3 behind a RIFF header that declares 8-bit PCM and a `data`
//! chunk of length zero; the MP3 stream (sometimes after an ID3v2 tag)
//! follows that chunk header to the end of the file. Others are PCM, or
//! declare MPEG Layer 3 (format tag 85) with a real `data` length. The
//! layout of the MPEG audio frame header and of the Xing/Info and VBRI
//! frame counts is the one the MPEG-1 and MPEG-2 audio standards and the
//! encoders' published notes describe.

use serde::Serialize;

/// How many bytes of a file [`duration`] wants: enough for the RIFF header,
/// a small ID3 tag and the first MP3 frame.
pub const HEAD: usize = 8192;

/// What the head of a voice file says about it.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Audio {
    /// The duration in seconds.
    pub seconds: f64,
    /// `pcm`, or `mp3` for an MPEG Layer III stream.
    pub encoding: &'static str,
    /// How the duration was found: `pcm` (data length over byte rate),
    /// `frames` (the frame count an Xing, Info or VBRI header gives), or
    /// `bitrate` (the stream's length over the first frame's bitrate, exact
    /// for a constant-bitrate stream).
    pub method: &'static str,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn be32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// The duration of a voice file from its first bytes (`head`, at least
/// [`HEAD`] of them or the whole file) and its length. `None` when the head
/// is not a RIFF WAVE this reads.
pub fn duration(head: &[u8], file_len: u64) -> Option<Audio> {
    if head.get(..4)? != b"RIFF" || head.get(8..12)? != b"WAVE" {
        return None;
    }
    let mut at = 12;
    let mut fmt: Option<(u16, u32)> = None;
    while at + 8 <= head.len() {
        let id = &head[at..at + 4];
        let len = u32_at(head, at + 4)?;
        let body = at + 8;
        if id == b"fmt " {
            fmt = Some((u16_at(head, body)?, u32_at(head, body + 8)?));
        } else if id == b"data" {
            let (tag, byte_rate) = fmt?;
            let end = if len == 0 {
                file_len
            } else {
                (body as u64 + u64::from(len)).min(file_len)
            };
            if let Some(a) = mp3(head, body, end) {
                return Some(a);
            }
            if tag == 1 && len > 0 && byte_rate > 0 {
                return Some(Audio {
                    seconds: f64::from(len) / f64::from(byte_rate),
                    encoding: "pcm",
                    method: "pcm",
                });
            }
            return None;
        }
        at = body + len as usize + (len as usize & 1);
    }
    None
}

/// An MP3 stream from `start` to `end` of the file, `head` holding its
/// beginning.
fn mp3(head: &[u8], start: usize, end: u64) -> Option<Audio> {
    let mut at = start;
    if head.get(at..at + 3)? == b"ID3" {
        let size = head.get(at + 6..at + 10)?;
        let size = size
            .iter()
            .fold(0usize, |n, &b| (n << 7) | usize::from(b & 0x7f));
        let footer = head.get(at + 5)? & 0x10 != 0;
        at += 10 + size + if footer { 10 } else { 0 };
    }
    // Some encoders pad before the first frame.
    while head.get(at) == Some(&0) {
        at += 1;
    }
    let frame = Frame::parse(head.get(at..at + 4)?)?;
    let samples = frame.samples();
    if let Some(frames) = frame.frame_count(&head[at..]) {
        return Some(Audio {
            seconds: f64::from(frames) * f64::from(samples) / f64::from(frame.rate),
            encoding: "mp3",
            method: "frames",
        });
    }
    let bytes = end.checked_sub(at as u64)?;
    Some(Audio {
        seconds: bytes as f64 * 8.0 / (f64::from(frame.kbps) * 1000.0),
        encoding: "mp3",
        method: "bitrate",
    })
}

/// An MPEG audio Layer III frame header.
struct Frame {
    /// 1 for MPEG-1, 2 for MPEG-2, 25 for MPEG-2.5.
    version: u8,
    kbps: u32,
    rate: u32,
    mono: bool,
}

impl Frame {
    fn parse(b: &[u8]) -> Option<Frame> {
        let h = be32_at(b, 0)?;
        if h >> 21 != 0x7ff {
            return None;
        }
        let version = match (h >> 19) & 3 {
            0 => 25,
            2 => 2,
            3 => 1,
            _ => return None,
        };
        // Layer III only.
        if (h >> 17) & 3 != 1 {
            return None;
        }
        let bitrate = ((h >> 12) & 0xf) as usize;
        let rate = ((h >> 10) & 3) as usize;
        const V1: [u32; 15] = [
            0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320,
        ];
        const V2: [u32; 15] = [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160];
        let kbps = *if version == 1 { &V1 } else { &V2 }.get(bitrate)?;
        let rate = match version {
            1 => [44100, 48000, 32000],
            2 => [22050, 24000, 16000],
            _ => [11025, 12000, 8000],
        }
        .get(rate)
        .copied()?;
        if kbps == 0 {
            return None;
        }
        Some(Frame {
            version,
            kbps,
            rate,
            mono: (h >> 6) & 3 == 3,
        })
    }

    fn samples(&self) -> u32 {
        if self.version == 1 { 1152 } else { 576 }
    }

    /// The frame count of an Xing or Info header after the side
    /// information, or of a VBRI header 32 bytes after the frame header.
    fn frame_count(&self, frame: &[u8]) -> Option<u32> {
        let side = match (self.version == 1, self.mono) {
            (true, false) => 32,
            (true, true) | (false, false) => 17,
            (false, true) => 9,
        };
        let x = 4 + side;
        if let Some(tag) = frame.get(x..x + 4)
            && (tag == b"Xing" || tag == b"Info")
        {
            let flags = be32_at(frame, x + 4)?;
            return (flags & 1 != 0)
                .then(|| be32_at(frame, x + 8))
                .flatten()
                .filter(|&n| n > 0);
        }
        if frame.get(36..40)? == b"VBRI" {
            return be32_at(frame, 36 + 14).filter(|&n| n > 0);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn riff(fmt_tag: u16, byte_rate: u32, data_len: u32, payload: &[u8]) -> Vec<u8> {
        let mut b = b"RIFF\0\0\0\0WAVEfmt ".to_vec();
        b.extend(16u32.to_le_bytes());
        b.extend(fmt_tag.to_le_bytes());
        b.extend(1u16.to_le_bytes());
        b.extend(22050u32.to_le_bytes());
        b.extend(byte_rate.to_le_bytes());
        b.extend(2u16.to_le_bytes());
        b.extend(16u16.to_le_bytes());
        b.extend(b"data");
        b.extend(data_len.to_le_bytes());
        b.extend(payload);
        b
    }

    #[test]
    fn pcm_is_its_data_length_over_its_byte_rate() {
        let b = riff(1, 44100, 88200, &[1; 64]);
        let a = duration(&b, 44 + 88200).unwrap();
        assert_eq!((a.seconds, a.encoding, a.method), (2.0, "pcm", "pcm"));
    }

    /// An MPEG-1 Layer III frame header at 32 kHz, 48 kbit/s, mono, then
    /// the side information and, when `frames` is set, an Info header.
    fn mp3_frame(frames: Option<u32>) -> Vec<u8> {
        let mut f = vec![0xff, 0xfb, 0x38, 0xc4];
        f.extend([0u8; 17]);
        if let Some(n) = frames {
            f.extend(b"Info");
            f.extend(0x0fu32.to_be_bytes());
            f.extend(n.to_be_bytes());
        }
        f.resize(144, 0);
        f
    }

    #[test]
    fn mp3_behind_a_zero_length_data_chunk_counts_its_frames() {
        let b = riff(1, 22050, 0, &mp3_frame(Some(97)));
        let a = duration(&b, 21226).unwrap();
        assert_eq!((a.encoding, a.method), ("mp3", "frames"));
        assert!((a.seconds - 97.0 * 1152.0 / 32000.0).abs() < 1e-9);
    }

    #[test]
    fn mp3_without_a_frame_count_goes_by_its_bitrate() {
        let b = riff(1, 22050, 0, &mp3_frame(None));
        let len = b.len() as u64 - mp3_frame(None).len() as u64 + 6000;
        let a = duration(&b, len).unwrap();
        assert_eq!(a.method, "bitrate");
        assert!((a.seconds - 6000.0 * 8.0 / 48000.0).abs() < 1e-9);
    }

    #[test]
    fn an_id3_tag_before_the_stream_is_skipped() {
        let mut payload = b"ID3\x03\0\0\0\0\0\x05".to_vec();
        payload.extend([0u8; 5]);
        payload.extend(mp3_frame(Some(10)));
        let b = riff(1, 22050, 0, &payload);
        let a = duration(&b, 9999).unwrap();
        assert!((a.seconds - 10.0 * 1152.0 / 32000.0).abs() < 1e-9);
    }

    #[test]
    fn not_a_wave_is_none() {
        assert_eq!(duration(b"OggS\0\0\0\0\0\0\0\0", 12), None);
        assert_eq!(duration(b"RIFF", 4), None);
    }
}
