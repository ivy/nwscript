//! The install as `nwscript`'s [`Resolver`]: includes and `nwscript.nss`
//! answered from the engine's own search order, so a script compiles against
//! what the game would load.

use std::io;
use std::sync::Arc;

use nwscript::{InputRecord, Resolved, Resolver, sha256_hex};
use rakata_res::{ResId, ResRef, ResType};

use crate::{Install, ModuleName};

impl Install {
    /// Make the resolver search this module's capsules as well as Override
    /// and the game's own files. Module sources are not shipped, so this
    /// matters only for a build that puts them there.
    pub fn set_resolver_module(&mut self, module: Option<ModuleName>) {
        self.resolver_module = module;
    }

    fn resolve_source(&mut self, name: &str, role: &str) -> io::Result<Option<Resolved>> {
        let Ok(resref) = ResRef::new(name) else {
            // A name longer than a resref cannot be in the install.
            return Ok(None);
        };
        let id = ResId::new(resref, ResType::Nss);
        let module = self.resolver_module.clone();
        let found = self.read(id, module.as_ref()).map_err(io::Error::other)?;
        let Some((location, bytes)) = found else {
            return Ok(None);
        };
        let bytes: Arc<[u8]> = bytes.into();
        let path = location.to_string();
        self.inputs.push(InputRecord {
            role: role.into(),
            path: path.clone(),
            sha256: sha256_hex(&bytes),
        });
        Ok(Some(Resolved { path, bytes }))
    }
}

impl Resolver for Install {
    fn include(&mut self, name: &str) -> io::Result<Option<Resolved>> {
        self.resolve_source(name, "include")
    }

    fn nwscript(&mut self) -> io::Result<Option<Resolved>> {
        self.resolve_source("nwscript", "nwscript")
    }

    fn inputs(&self) -> &[InputRecord] {
        &self.inputs
    }
}
