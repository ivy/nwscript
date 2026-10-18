//! A `changes.ini` of arbitrary bytes, read as the installer reads it: the
//! INI, the changes, the namespaces and the plan built from the changes.
//! Each reports what it cannot use as issues and none panics. The same
//! bytes are read as an SSF.

#![no_main]

use libfuzzer_sys::fuzz_target;
use rakata_patch::ssf::Ssf;
use rakata_patch::{Changes, Ini, Namespaces, Plan};

fuzz_target!(|data: &[u8]| {
    let mut issues = Vec::new();
    let ini = Ini::parse(data, 0, &mut issues);
    let changes = Changes::parse(&ini, 0, &mut issues);
    let plan = Plan::build(&changes, 0, &mut issues);
    let _ = plan.to_string();
    let _ = Namespaces::parse(&ini, 0, &mut issues);
    for i in &issues {
        let _ = (i.is_error(), i.to_string());
    }
    let _ = Ssf::parse(data);
});
