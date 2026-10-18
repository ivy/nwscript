//! What both text formats share: lines, numbers, and the record of how a
//! file read departs from the form this crate writes, so that a file read
//! and written again is the bytes it came from.

use std::collections::BTreeMap;

use crate::{Error, Result};

/// One line of a file: its text without the ending, and its bytes with it.
pub(crate) struct Line<'a> {
    /// From one.
    pub number: usize,
    pub text: &'a str,
    pub raw: &'a str,
}

impl Line<'_> {
    pub fn tokens(&self) -> Vec<&str> {
        self.text.split_ascii_whitespace().collect()
    }

    pub fn is_blank(&self) -> bool {
        self.text.trim_ascii().is_empty()
    }

    pub fn is_indented(&self) -> bool {
        self.text.starts_with([' ', '\t'])
    }
}

/// The bytes as ASCII text, split into lines that keep their endings, and
/// the ending of the first line, which the writer uses throughout.
pub(crate) fn lines(bytes: &[u8]) -> Result<(Vec<Line<'_>>, Newline)> {
    if let Some(offset) = bytes.iter().position(|b| !b.is_ascii()) {
        return Err(Error::NotAscii {
            offset,
            byte: bytes[offset],
        });
    }
    let text = std::str::from_utf8(bytes).expect("ASCII is UTF-8");
    let mut out = Vec::new();
    for (i, raw) in text.split_inclusive('\n').enumerate() {
        let body = raw.strip_suffix('\n').unwrap_or(raw);
        let body = body.strip_suffix('\r').unwrap_or(body);
        out.push(Line {
            number: i + 1,
            text: body,
            raw,
        });
    }
    let newline = match out.first() {
        Some(l) if l.raw.ends_with("\r\n") || !l.raw.ends_with('\n') => Newline::CrLf,
        Some(_) => Newline::Lf,
        None => Newline::CrLf,
    };
    Ok((out, newline))
}

/// A line ending.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Newline {
    /// `\r\n`, which every vanilla file uses.
    #[default]
    CrLf,
    Lf,
}

impl Newline {
    pub fn as_str(self) -> &'static str {
        match self {
            Newline::CrLf => "\r\n",
            Newline::Lf => "\n",
        }
    }
}

/// How a file read departs from the form the writer produces, line by line:
/// for each departing line, by its position among the lines written, the
/// line the writer would produce for what was read and the bytes that were
/// there. The writer puts the bytes back wherever it produces that same
/// line in that position, so an edit elsewhere keeps them and an edit to
/// the line itself replaces them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Style {
    pub newline: Newline,
    departures: BTreeMap<usize, (String, String)>,
}

impl Style {
    pub fn new(newline: Newline) -> Self {
        Style {
            newline,
            departures: BTreeMap::new(),
        }
    }

    /// Note that line `index` was `raw` where the writer would produce
    /// `canonical`.
    pub fn record(&mut self, index: usize, canonical: &str, raw: &str) {
        let written = format!("{canonical}{}", self.newline.as_str());
        if written != raw {
            self.departures
                .insert(index, (canonical.to_owned(), raw.to_owned()));
        }
    }

    /// Append line `index`, whose canonical form is `canonical`.
    pub fn emit(&self, out: &mut String, index: usize, canonical: &str) {
        match self.departures.get(&index) {
            Some((c, raw)) if c == canonical => out.push_str(raw),
            _ => {
                out.push_str(canonical);
                out.push_str(self.newline.as_str());
            }
        }
    }

    /// How many lines depart from the written form.
    pub fn departures(&self) -> usize {
        self.departures.len()
    }
}

/// Lines written in order, each put back as read where the [`Style`]
/// recorded a departure for it.
pub(crate) struct Writer<'a> {
    style: &'a Style,
    out: String,
    index: usize,
}

impl<'a> Writer<'a> {
    pub fn new(style: &'a Style) -> Self {
        Writer {
            style,
            out: String::new(),
            index: 0,
        }
    }

    pub fn line(&mut self, canonical: &str) {
        self.style.emit(&mut self.out, self.index, canonical);
        self.index += 1;
    }

    pub fn finish(self) -> String {
        self.out
    }
}

/// A number as the files spell it: C's `%g` (six significant digits,
/// trailing zeros dropped, an exponent below 1e-4 or from 1e6 with at least
/// three digits), with `.0` appended where that leaves an integer. Every
/// coordinate in the vanilla files is spelled so. `None` for a value that
/// is not finite.
pub fn format_float(v: f32) -> Option<String> {
    if !v.is_finite() {
        return None;
    }
    let d = f64::from(v);
    if d == 0.0 {
        return Some(if v.is_sign_negative() { "-0.0" } else { "0.0" }.to_owned());
    }
    let sci = format!("{d:.5e}");
    let (mantissa, exp) = sci.split_once('e').expect("`{:e}` writes an exponent");
    let exp: i32 = exp.parse().expect("an integer exponent");
    let mut s = if !(-4..6).contains(&exp) {
        format!(
            "{}e{}{:03}",
            strip_zeros(mantissa),
            if exp < 0 { '-' } else { '+' },
            exp.unsigned_abs()
        )
    } else {
        let decimals = usize::try_from(5 - exp).expect("exp is below 6");
        strip_zeros(&format!("{d:.decimals$}")).to_owned()
    };
    if !s.contains(['.', 'e']) {
        s.push_str(".0");
    }
    Some(s)
}

fn strip_zeros(s: &str) -> &str {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.')
    } else {
        s
    }
}

pub(crate) fn parse_float(token: &str, line: usize) -> Result<f32> {
    match token.parse::<f32>() {
        Ok(v) if v.is_finite() => Ok(v),
        _ => Err(Error::BadNumber {
            line,
            text: token.to_owned(),
        }),
    }
}

pub(crate) fn parse_count(token: &str, line: usize) -> Result<usize> {
    token.parse::<usize>().map_err(|_| Error::BadNumber {
        line,
        text: token.to_owned(),
    })
}

/// A float for writing, or the error naming it.
pub(crate) fn float(v: f32, what: impl FnOnce() -> String) -> Result<String> {
    format_float(v).ok_or_else(|| Error::Unwritable {
        what: what(),
        reason: "the value is not finite",
    })
}

/// A name for writing: one token of printable ASCII.
pub(crate) fn name(s: &str, what: impl FnOnce() -> String) -> Result<&str> {
    if s.is_empty() || !s.bytes().all(|b| b.is_ascii_graphic()) {
        return Err(Error::Unwritable {
            what: what(),
            reason: "a name is one or more printable ASCII characters without whitespace",
        });
    }
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::format_float;

    // 0.707107 is a value as the files spell it, not an approximation of 1/√2.
    #[allow(clippy::approx_constant)]
    #[test]
    fn floats_are_spelled_as_the_vanilla_files_spell_them() {
        for (v, s) in [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (45.16, "45.16"),
            (39.4685, "39.4685"),
            (-0.0407561, "-0.0407561"),
            (0.707107, "0.707107"),
            (130.475, "130.475"),
            (9.76562e-6, "9.76562e-006"),
            (-4.37114e-7, "-4.37114e-007"),
            (1234567.0, "1.23457e+006"),
            (100000.0, "100000.0"),
            (0.0001, "0.0001"),
        ] {
            assert_eq!(format_float(v).as_deref(), Some(s), "{v}");
        }
        assert_eq!(format_float(f32::NAN), None);
        assert_eq!(format_float(f32::INFINITY), None);
    }
}
