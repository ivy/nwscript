//! `GLOBALVARS.res`: the campaign's global variables by name. The layout is
//! in `docs/formats/save.md`, "Globals".

use rakata_gff::Gff;

use crate::Result;
use crate::fields::At;

/// A position and a facing, as `GetGlobalLocation` returns them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Location {
    /// x, y, z.
    pub position: [f32; 3],
    /// The facing vector, x, y, z.
    pub orientation: [f32; 3],
}

/// The value of one global.
#[derive(Debug, Clone, PartialEq)]
pub enum GlobalValue {
    Boolean(bool),
    /// A global number is one signed byte.
    Number(i8),
    String(String),
    Location(Location),
}

impl GlobalValue {
    /// The type's name as the scripts spell it: `boolean`, `number`,
    /// `string`, `location`.
    pub const fn kind(&self) -> &'static str {
        match self {
            GlobalValue::Boolean(_) => "boolean",
            GlobalValue::Number(_) => "number",
            GlobalValue::String(_) => "string",
            GlobalValue::Location(_) => "location",
        }
    }
}

/// One global: its name as the file spells it, and its value.
#[derive(Debug, Clone, PartialEq)]
pub struct Global {
    pub name: String,
    pub value: GlobalValue,
}

/// Every global of a save, booleans first, then numbers, strings and
/// locations, each in the file's catalogue order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Globals {
    pub vars: Vec<Global>,
}

impl Globals {
    /// The globals in a `GVT ` GFF. `member` names it for errors.
    pub fn from_gff(gff: &Gff, member: &str) -> Result<Self> {
        let at = At::new(member, "", &gff.root);
        let mut vars = Vec::new();

        let bits = at.void("ValBoolean")?;
        for (i, e) in at.list("CatBoolean")?.iter().enumerate() {
            let name = At::new(member, "CatBoolean", e).string("Name")?;
            // Most significant bit first within each byte.
            let set = bits.get(i / 8).is_some_and(|b| b & (0x80 >> (i % 8)) != 0);
            vars.push(Global {
                name,
                value: GlobalValue::Boolean(set),
            });
        }

        let numbers = at.void("ValNumber")?;
        for (i, e) in at.list("CatNumber")?.iter().enumerate() {
            let name = At::new(member, "CatNumber", e).string("Name")?;
            let n = numbers.get(i).map_or(0, |&b| b as i8);
            vars.push(Global {
                name,
                value: GlobalValue::Number(n),
            });
        }

        let strings = at.list("ValString")?;
        for (i, e) in at.list("CatString")?.iter().enumerate() {
            let name = At::new(member, "CatString", e).string("Name")?;
            let text = match strings.get(i) {
                Some(s) => At::new(member, "ValString", s).string("String")?,
                None => String::new(),
            };
            vars.push(Global {
                name,
                value: GlobalValue::String(text),
            });
        }

        let locations = at.void("ValLocation")?;
        for (i, e) in at.list("CatLocation")?.iter().enumerate() {
            let name = At::new(member, "CatLocation", e).string("Name")?;
            let f = |k: usize| {
                let o = i * 24 + k * 4;
                locations
                    .get(o..o + 4)
                    .map_or(0.0, |b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            };
            vars.push(Global {
                name,
                value: GlobalValue::Location(Location {
                    position: [f(0), f(1), f(2)],
                    orientation: [f(3), f(4), f(5)],
                }),
            });
        }
        Ok(Globals { vars })
    }

    /// The global with this name, compared without regard to case, as the
    /// engine compares them.
    pub fn get(&self, name: &str) -> Option<&Global> {
        self.vars.iter().find(|g| g.name.eq_ignore_ascii_case(name))
    }

    /// The globals whose names contain `pattern`, without regard to case.
    pub fn matching<'a>(&'a self, pattern: &'a str) -> impl Iterator<Item = &'a Global> + 'a {
        let p = pattern.to_ascii_lowercase();
        self.vars
            .iter()
            .filter(move |g| g.name.to_ascii_lowercase().contains(&p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rakata_gff::{Label, Struct, Value};

    fn named(name: &str) -> Struct {
        let mut s = Struct::new(0);
        s.push(Label::new("Name").unwrap(), Value::string(name));
        s
    }

    fn gvt() -> Gff {
        let mut g = Gff::new(*b"GVT ");
        let l = |t: &str| Label::new(t).unwrap();
        let r = &mut g.root;
        r.push(
            l("CatBoolean"),
            Value::List((0..10).map(|i| named(&format!("B{i}"))).collect()),
        );
        r.push(l("CatNumber"), Value::List(vec![named("N0"), named("N1")]));
        r.push(l("CatLocation"), Value::List(vec![named("L0")]));
        r.push(l("CatString"), Value::List(vec![named("S0")]));
        // B0 and B9 set: 1000_0000 0100_0000.
        r.push(l("ValBoolean"), Value::Void(vec![0x80, 0x40]));
        r.push(l("ValNumber"), Value::Void(vec![7, 0xFE]));
        let mut loc = Vec::new();
        for f in [1.0f32, 2.0, 3.0, 0.0, 1.0, 0.0] {
            loc.extend_from_slice(&f.to_le_bytes());
        }
        r.push(l("ValLocation"), Value::Void(loc));
        let mut s = Struct::new(0);
        s.push(l("String"), Value::string("hello"));
        r.push(l("ValString"), Value::List(vec![s]));
        g
    }

    #[test]
    fn booleans_read_most_significant_bit_first_and_numbers_signed() {
        let g = Globals::from_gff(&gvt(), "GLOBALVARS.res").unwrap();
        let v = |n: &str| g.get(n).unwrap().value.clone();
        assert_eq!(v("b0"), GlobalValue::Boolean(true));
        assert_eq!(v("B1"), GlobalValue::Boolean(false));
        assert_eq!(v("B8"), GlobalValue::Boolean(false));
        assert_eq!(v("B9"), GlobalValue::Boolean(true));
        assert_eq!(v("N0"), GlobalValue::Number(7));
        assert_eq!(v("N1"), GlobalValue::Number(-2));
        assert_eq!(v("S0"), GlobalValue::String("hello".into()));
        assert_eq!(
            v("L0"),
            GlobalValue::Location(Location {
                position: [1.0, 2.0, 3.0],
                orientation: [0.0, 1.0, 0.0]
            })
        );
        assert_eq!(g.matching("b").count(), 10);
    }

    #[test]
    fn a_missing_catalogue_names_its_field() {
        let mut gff = gvt();
        gff.root.remove("CatNumber");
        let e = Globals::from_gff(&gff, "GLOBALVARS.res").unwrap_err();
        assert_eq!(e.code(), crate::codes::BAD_SHAPE);
        assert!(e.message().contains("`CatNumber`"), "{}", e.message());
    }
}
