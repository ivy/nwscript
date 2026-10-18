//! `Save`: a save directory opened, its archives listed, its GFFs read.

use std::path::{Path, PathBuf};

use rakata_archive::Erf;
use rakata_gff::Gff;
use rakata_res::ResType;

use crate::creature::{self, Creature, ObjectRef, Objects, Place};
use crate::fields::{At, int, string, u32_field};
use crate::globals::Globals;
use crate::{Error, Result};

/// The archive of a save's state.
pub(crate) const SAVEGAME: &str = "SAVEGAME.sav";
const GLOBALVARS: &str = "GLOBALVARS";
const PARTYTABLE: &str = "PARTYTABLE";
const SAVENFO: &str = "savenfo";
const REPUTE: &str = "REPUTE";
const AVAILNPC: &str = "AVAILNPC";

/// A saved game: the directory `Saves/NNNNNN - GameN/`.
///
/// Opening reads `SAVEGAME.sav` and the small GFFs beside it; each module's
/// state is read when asked for. Every GFF is read through `rakata-gff`, so
/// any of them written back with [`Gff::to_bytes`] is the bytes it came from.
pub struct Save {
    pub(crate) dir: PathBuf,
    savegame: Vec<u8>,
    pub(crate) globals: Option<(String, Gff)>,
    pub(crate) party: Option<(String, Gff)>,
    info: Option<Gff>,
}

/// `savenfo.res`: what the load screen shows.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Info {
    pub area_name: Option<String>,
    /// The module the player was in, the stem of one of [`Save::modules`].
    pub last_module: Option<String>,
    pub save_name: Option<String>,
    pub time_played: Option<i64>,
}

/// One party slot of `PARTYTABLE.res`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// The NPC number: the `n` of `AVAILNPC<n>.utc`.
    pub npc: i64,
    pub is_leader: bool,
    /// The tag of `AVAILNPC<n>.utc`, as saved.
    pub tag: Option<String>,
}

/// One entry of the table of NPCs the player may choose from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Available {
    pub npc: usize,
    pub available: bool,
    pub selectable: bool,
}

/// `PARTYTABLE.res`: who is in the party.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Party {
    /// The members travelling with the player, in table order.
    pub members: Vec<Member>,
    pub available: Vec<Available>,
    pub gold: Option<i64>,
    pub xp_pool: Option<i64>,
    /// The NPC the player controls; -1 for the player character.
    pub controlled: Option<i64>,
}

/// One module's state: the nested `<module>.sav` and the GFFs in it.
pub struct Module {
    /// The module's name, the nested archive's stem.
    pub name: String,
    /// The area's instances (`<area>.git`) and that member's name.
    pub git: (String, Gff),
    /// `Module.ifo`, with the event queue and the player list.
    pub ifo: Option<(String, Gff)>,
    /// The area (`<area>.are`).
    pub are: Option<(String, Gff)>,
}

/// One pending event of a module's `EventQueue`.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub day: Option<i64>,
    /// Milliseconds into the day.
    pub time: Option<i64>,
    pub object: ObjectRef,
    pub caller: ObjectRef,
    /// The engine's event id. Its names live in the engine.
    pub event_id: Option<i64>,
    /// The name of the script situation the event runs, if it carries one.
    pub script: Option<String>,
}

/// A GFF member of a save, wherever it lies: a loose file beside
/// `SAVEGAME.sav`, a member of it, or a member of a module's archive in it.
pub struct GffMember {
    /// Where it is: `GLOBALVARS.res`, `SAVEGAME.sav/AVAILNPC0.utc`,
    /// `SAVEGAME.sav/danm14ac.sav/m14ac.git`.
    pub location: String,
    /// The bytes as the save holds them.
    pub bytes: Vec<u8>,
}

impl Save {
    /// The save in `dir`. Fails when the directory holds no `SAVEGAME.sav`,
    /// or when it or a GFF beside it does not read.
    pub fn open(dir: impl AsRef<Path>) -> Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        let Some(path) = child_ci(&dir, SAVEGAME) else {
            return Err(Error::NotASave { dir });
        };
        let savegame = std::fs::read(&path).map_err(|source| Error::Io {
            path: path.clone(),
            source,
        })?;
        let mut save = Save {
            dir,
            savegame,
            globals: None,
            party: None,
            info: None,
        };
        save.outer()?;
        save.globals = save.small_gff(GLOBALVARS)?;
        save.party = save.small_gff(PARTYTABLE)?;
        save.info = save.small_gff(SAVENFO)?.map(|(_, g)| g);
        Ok(save)
    }

    /// The directory the save was opened from.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub(crate) fn outer(&self) -> Result<Erf<'_>> {
        Erf::parse(&self.savegame).map_err(|source| Error::Archive {
            member: SAVEGAME.to_owned(),
            source,
        })
    }

    /// A `.res` GFF beside `SAVEGAME.sav`, else inside it.
    fn small_gff(&self, stem: &str) -> Result<Option<(String, Gff)>> {
        let file = format!("{stem}.res");
        if let Some(path) = child_ci(&self.dir, &file) {
            let bytes = std::fs::read(&path).map_err(|source| Error::Io {
                path: path.clone(),
                source,
            })?;
            return parse_gff(&bytes, &file).map(|g| Some((file, g)));
        }
        let outer = self.outer()?;
        match member(&outer, stem, ResType::Res) {
            Some(bytes) => {
                let at = format!("{SAVEGAME}/{file}");
                parse_gff(bytes, &at).map(|g| Some((at, g)))
            }
            None => Ok(None),
        }
    }

    /// `savenfo.res`, empty when the save has none.
    pub fn info(&self) -> Info {
        let Some(g) = &self.info else {
            return Info::default();
        };
        let r = &g.root;
        Info {
            area_name: string(r, "AREANAME"),
            last_module: string(r, "LASTMODULE"),
            save_name: string(r, "SAVEGAMENAME").filter(|s| !s.is_empty()),
            time_played: int(r, "TIMEPLAYED"),
        }
    }

    /// The globals, by name and type. Empty when the save has no
    /// `GLOBALVARS.res`.
    pub fn globals(&self) -> Result<Globals> {
        match &self.globals {
            Some((at, g)) => Globals::from_gff(g, at),
            None => Ok(Globals::default()),
        }
    }

    /// The `GLOBALVARS.res` GFF itself, and where it lies.
    pub fn globals_gff(&self) -> Option<(&str, &Gff)> {
        self.globals.as_ref().map(|(a, g)| (a.as_str(), g))
    }

    /// The party table, each member named by the tag of its `AVAILNPC`.
    pub fn party(&self) -> Result<Party> {
        let Some((at, g)) = &self.party else {
            return Ok(Party::default());
        };
        let root = At::new(at, "", &g.root);
        let npcs = self.party_gffs()?;
        let tag_of = |n: i64| {
            npcs.iter()
                .find(|(i, _)| i64::from(*i) == n)
                .and_then(|(_, g)| string(&g.root, "Tag"))
        };
        let members = root
            .list("PT_MEMBERS")?
            .iter()
            .map(|m| {
                let npc = int(m, "PT_MEMBER_ID").unwrap_or(-1);
                Member {
                    npc,
                    is_leader: int(m, "PT_IS_LEADER").unwrap_or(0) != 0,
                    tag: tag_of(npc),
                }
            })
            .collect();
        let available = root
            .list("PT_AVAIL_NPCS")?
            .iter()
            .enumerate()
            .map(|(npc, a)| Available {
                npc,
                available: int(a, "PT_NPC_AVAIL").unwrap_or(0) != 0,
                selectable: int(a, "PT_NPC_SELECT").unwrap_or(0) != 0,
            })
            .collect();
        Ok(Party {
            members,
            available,
            gold: int(&g.root, "PT_GOLD"),
            xp_pool: int(&g.root, "PT_XP_POOL"),
            controlled: int(&g.root, "PT_CONTROLLED_NP"),
        })
    }

    /// Every `AVAILNPC<n>.utc`, by NPC number, in archive order.
    pub fn party_gffs(&self) -> Result<Vec<(u32, Gff)>> {
        let outer = self.outer()?;
        let mut out = Vec::new();
        for m in outer.members() {
            if m.id.ty != ResType::Utc {
                continue;
            }
            let name = m.id.name.to_string();
            let Some(n) = strip_prefix_ci(&name, AVAILNPC).and_then(|n| n.parse().ok()) else {
                continue;
            };
            let at = format!("{SAVEGAME}/{name}.utc");
            out.push((n, parse_gff(m.data(), &at)?));
        }
        Ok(out)
    }

    /// The names of the modules the save holds state for, in archive order:
    /// the stems of the nested `.sav` members.
    pub fn modules(&self) -> Result<Vec<String>> {
        Ok(self
            .outer()?
            .members()
            .iter()
            .filter(|m| m.id.ty == ResType::Sav)
            .map(|m| m.id.name.to_string())
            .collect())
    }

    /// One module's state, by name, compared without regard to case.
    pub fn module(&self, name: &str) -> Result<Module> {
        let outer = self.outer()?;
        let Some(m) = outer
            .members()
            .iter()
            .find(|m| m.id.ty == ResType::Sav && m.id.name.to_string().eq_ignore_ascii_case(name))
        else {
            return Err(Error::NoSuchModule {
                name: name.to_owned(),
                available: self.modules()?,
            });
        };
        let stem = m.id.name.to_string();
        let here = format!("{SAVEGAME}/{stem}.sav");
        let inner = Erf::parse(m.data()).map_err(|source| Error::Archive {
            member: here.clone(),
            source,
        })?;
        let mut git = None;
        let mut ifo = None;
        let mut are = None;
        for c in inner.members() {
            let slot = match c.id.ty {
                ResType::Git => &mut git,
                ResType::Ifo => &mut ifo,
                ResType::Are => &mut are,
                _ => continue,
            };
            if slot.is_some() {
                continue;
            }
            let file = format!("{}.{}", c.id.name, c.id.ty.extension().unwrap_or("res"));
            let g = parse_gff(c.data(), &format!("{here}/{file}"))?;
            *slot = Some((file, g));
        }
        let git = git.ok_or_else(|| Error::BadShape {
            member: here,
            path: String::new(),
            expected: "a module state holding a GIT",
        })?;
        Ok(Module {
            name: stem,
            git,
            ifo,
            are,
        })
    }

    /// The faction names of `REPUTE.fac`, by faction id.
    fn factions(&self) -> Result<Vec<String>> {
        let outer = self.outer()?;
        let Some(bytes) = member(&outer, REPUTE, ResType::Fac) else {
            return Ok(Vec::new());
        };
        let at = format!("{SAVEGAME}/{REPUTE}.fac");
        let g = parse_gff(bytes, &at)?;
        Ok(At::new(&at, "", &g.root)
            .list("FactionList")?
            .iter()
            .map(|f| string(f, "FactionName").unwrap_or_default())
            .collect())
    }

    /// The creatures of one module (its GIT's `Creature List` and its IFO's
    /// `Mod_PlayerList`), or of every module when `module` is `None`, then
    /// the party members of `AVAILNPC<n>.utc`. A reference to another object
    /// is named from the module the creature is in; a party member's, from
    /// `module` or else the module the save was made in.
    pub fn creatures(&self, module: Option<&str>) -> Result<Vec<Creature>> {
        let factions = self.factions()?;
        let names: Vec<String> = match module {
            Some(m) => vec![m.to_owned()],
            None => self.modules()?,
        };
        let npcs = self.party_gffs()?;
        let mut party_objects = Objects::default();
        for (_, g) in &npcs {
            party_objects.add(&g.root);
        }
        let home = module
            .map(str::to_owned)
            .or_else(|| self.info().last_module);

        let mut out = Vec::new();
        for name in &names {
            let m = self.module(name)?;
            let mut objects = Objects::default();
            objects.add(&m.git.1.root);
            if let Some((_, ifo)) = &m.ifo {
                objects.add(&ifo.root);
            }
            if home
                .as_deref()
                .is_some_and(|h| h.eq_ignore_ascii_case(&m.name))
            {
                party_objects.add(&m.git.1.root);
                if let Some((_, ifo)) = &m.ifo {
                    party_objects.add(&ifo.root);
                }
            }
            for (member, gff, list) in [
                (Some(&m.git.0), Some(&m.git.1), "Creature List"),
                (
                    m.ifo.as_ref().map(|i| &i.0),
                    m.ifo.as_ref().map(|i| &i.1),
                    "Mod_PlayerList",
                ),
            ] {
                let (Some(member), Some(gff)) = (member, gff) else {
                    continue;
                };
                let entries = gff
                    .root
                    .get(list)
                    .and_then(rakata_gff::Value::as_list)
                    .unwrap_or(&[]);
                for (i, s) in entries.iter().enumerate() {
                    let place = Place::Module {
                        module: m.name.clone(),
                        member: member.clone(),
                        path: format!("{list}[{i}]"),
                    };
                    out.push(creature::read(s, place, &objects, &factions));
                }
            }
        }
        for (npc, g) in &npcs {
            let place = Place::PartyMember { npc: *npc };
            out.push(creature::read(&g.root, place, &party_objects, &factions));
        }
        Ok(out)
    }

    /// The creatures whose tag is `tag`, compared without regard to case,
    /// in the places [`Save::creatures`] searches.
    pub fn find_creatures(&self, tag: &str, module: Option<&str>) -> Result<Vec<Creature>> {
        Ok(self
            .creatures(module)?
            .into_iter()
            .filter(|c| c.tag.eq_ignore_ascii_case(tag))
            .collect())
    }

    /// The module's pending events, from its IFO's `EventQueue`, in queue
    /// order.
    pub fn events(&self, module: &str) -> Result<Vec<Event>> {
        let m = self.module(module)?;
        let Some((member, ifo)) = &m.ifo else {
            return Ok(Vec::new());
        };
        let mut objects = Objects::default();
        objects.add(&m.git.1.root);
        objects.add(&ifo.root);
        let at = format!("{SAVEGAME}/{}.sav/{member}", m.name);
        let Some(queue) = ifo.root.get("EventQueue") else {
            return Ok(Vec::new());
        };
        let queue = queue.as_list().ok_or_else(|| Error::BadShape {
            member: at,
            path: "EventQueue".to_owned(),
            expected: "a list",
        })?;
        Ok(queue
            .iter()
            .map(|e| Event {
                day: int(e, "Day"),
                time: int(e, "Time"),
                object: objects
                    .object(u32_field(e, "ObjectId").unwrap_or(creature::OBJECT_INVALID)),
                caller: objects
                    .object(u32_field(e, "CallerId").unwrap_or(creature::OBJECT_INVALID)),
                event_id: int(e, "EventId"),
                script: e
                    .get("EventData")
                    .and_then(rakata_gff::Value::as_struct)
                    .and_then(|d| string(d, "Name")),
            })
            .collect())
    }

    /// Every GFF the save holds, in the directory beside `SAVEGAME.sav`, in
    /// it, and in each module's archive in it, with its bytes as stored.
    /// The screenshot and any other file that is not a GFF are left out.
    pub fn gff_members(&self) -> Result<Vec<GffMember>> {
        let mut out = Vec::new();
        let mut files: Vec<PathBuf> = std::fs::read_dir(&self.dir)
            .map_err(|source| Error::Io {
                path: self.dir.clone(),
                source,
            })?
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.is_file())
            .collect();
        files.sort();
        for p in files {
            let bytes = std::fs::read(&p).map_err(|source| Error::Io {
                path: p.clone(),
                source,
            })?;
            if is_gff(&bytes) {
                let name = p.file_name().map(|n| n.to_string_lossy().into_owned());
                out.push(GffMember {
                    location: name.unwrap_or_default(),
                    bytes,
                });
            }
        }
        let outer = self.outer()?;
        for m in outer.members() {
            let file = format!("{}.{}", m.id.name, m.id.ty.extension().unwrap_or("res"));
            if m.id.ty == ResType::Sav {
                let here = format!("{SAVEGAME}/{file}");
                let inner = Erf::parse(m.data()).map_err(|source| Error::Archive {
                    member: here.clone(),
                    source,
                })?;
                for c in inner.members() {
                    if is_gff(c.data()) {
                        let f = format!("{}.{}", c.id.name, c.id.ty.extension().unwrap_or("res"));
                        out.push(GffMember {
                            location: format!("{here}/{f}"),
                            bytes: c.data().to_vec(),
                        });
                    }
                }
            } else if is_gff(m.data()) {
                out.push(GffMember {
                    location: format!("{SAVEGAME}/{file}"),
                    bytes: m.data().to_vec(),
                });
            }
        }
        Ok(out)
    }
}

/// Whether bytes begin as a GFF V3.2 does: four type bytes, then `V3.2`.
fn is_gff(bytes: &[u8]) -> bool {
    bytes.get(4..8) == Some(b"V3.2")
}

pub(crate) fn parse_gff(bytes: &[u8], member: &str) -> Result<Gff> {
    Gff::parse(bytes).map_err(|source| Error::Gff {
        member: member.to_owned(),
        source,
    })
}

fn member<'a>(erf: &'a Erf<'_>, stem: &str, ty: ResType) -> Option<&'a [u8]> {
    erf.members()
        .iter()
        .find(|m| m.id.ty == ty && m.id.name.to_string().eq_ignore_ascii_case(stem))
        .map(|m| m.data())
}

fn strip_prefix_ci<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
    let head = s.get(..prefix.len())?;
    head.eq_ignore_ascii_case(prefix)
        .then(|| &s[prefix.len()..])
}

/// The entry of `dir` named `name`, compared without regard to case.
fn child_ci(dir: &Path, name: &str) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .find(|e| e.file_name().to_string_lossy().eq_ignore_ascii_case(name))
        .map(|e| e.path())
}
