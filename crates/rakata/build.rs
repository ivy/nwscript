//! Computes the build identity ([`identity`]) and hands it to the crate as
//! `RAKATA_VERSION`, re-running when any tracked file or the repository's
//! metadata changes. A file created since the last build is noticed on the
//! next build that something else causes.

#[path = "build/identity.rs"]
mod identity;

use std::path::Path;

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let version = std::env::var("CARGO_PKG_VERSION").expect("cargo sets CARGO_PKG_VERSION");
    let checkout = identity::checkout(Path::new(&manifest_dir));
    if let Some(co) = &checkout {
        for p in identity::watched_paths(co) {
            println!("cargo:rerun-if-changed={}", p.display());
        }
    } else {
        println!("cargo:rerun-if-changed=build.rs");
    }
    let id = identity::identity(&version, checkout.as_ref());
    println!("cargo:rustc-env=RAKATA_VERSION={id}");
}
