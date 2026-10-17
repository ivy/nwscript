//! `decompile` on any program `decode` accepts, behind an entry stub: it
//! recovers what it can, reports the rest per function, and neither it nor
//! printing the result panics.

#![no_main]

use libfuzzer_sys::fuzz_target;
use nwscript_diag::FileId;
use nwscript_sema::EngineTable;
use std::sync::OnceLock;

/// A few routines of each shape: a value, a void, defaults, a vector, an action.
const ENGINE: &str = "int Random(int n);
void PrintString(string s);
void PrintFloat(float f, int w = 18, int d = 9);
string IntToString(int n);
void PrintInteger(int n);
object GetObject(object o = OBJECT_SELF, int n = -1);
void AssignCommand(object o, action a);
void DelayCommand(float f, action a);
vector Vector(float x = 0.0, float y = 0.0, float z = 0.0);
";

fn table() -> &'static EngineTable {
    static T: OnceLock<EngineTable> = OnceLock::new();
    T.get_or_init(|| EngineTable::from_nwscript(FileId(0), ENGINE.as_bytes()).0)
}

fuzz_target!(|data: &[u8]| {
    // The bytes as a program body, after the stub most scripts start with.
    let mut file = b"NCS V1.0\x42".to_vec();
    file.extend_from_slice(&((13 + 8 + data.len()) as u32).to_be_bytes());
    file.extend_from_slice(&[0x1e, 0x00, 0, 0, 0, 8, 0x20, 0x00]);
    file.extend_from_slice(data);
    if let Ok(program) = nwscript_ncs::decode(FileId(1), &file)
        && let Ok(d) = nwscript_decomp::decompile(&program, table())
    {
        let _ = d.source();
    }
});
