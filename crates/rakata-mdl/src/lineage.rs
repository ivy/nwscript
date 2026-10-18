//! Supermodel inheritance: a model's chain of supermodels loaded through a
//! caller's loader, and the animations and nodes it resolves.

use crate::animation::Animation;
use crate::error::{ChainError, Error, Result};
use crate::model::{FILE_HEADER_LEN, Model, Node, parse};

// The supermodel name's offset in the model data.
const MODEL_SUPER: usize = 0x88;

/// The supermodel an MDL names, read from its header alone; `None` when it
/// names none (`NULL` or empty).
pub fn supermodel_name(mdl: &[u8]) -> Result<Option<&str>> {
    let at = FILE_HEADER_LEN + MODEL_SUPER;
    let field = mdl.get(at..at + 32).ok_or(Error::Truncated {
        what: "supermodel name",
        offset: MODEL_SUPER,
        needed: 32,
        available: mdl.len().saturating_sub(FILE_HEADER_LEN),
    })?;
    if mdl[..4] != [0; 4] {
        let first = u32::from_le_bytes([mdl[0], mdl[1], mdl[2], mdl[3]]);
        return Err(Error::NotBinary { first });
    }
    let end = field.iter().position(|&c| c == 0).ok_or(Error::BadName {
        what: "supermodel name",
        offset: MODEL_SUPER,
    })?;
    let s = std::str::from_utf8(&field[..end])
        .ok()
        .filter(|s| s.is_ascii())
        .ok_or(Error::BadName {
            what: "supermodel name",
            offset: MODEL_SUPER,
        })?;
    Ok((!s.is_empty() && !s.eq_ignore_ascii_case("NULL")).then_some(s))
}

/// One model's files, as a loader gave them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelFiles {
    /// The name the model was asked for by.
    pub name: String,
    /// Its MDL.
    pub mdl: Vec<u8>,
    /// Its MDX (empty when it has none).
    pub mdx: Vec<u8>,
}

/// A model and its supermodels' files, the model first, each followed by
/// the supermodel it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chain {
    /// The files, the model first.
    pub files: Vec<ModelFiles>,
    /// The supermodel the last model names when the loader had no files
    /// for it (three vanilla models name one the game does not ship).
    pub missing: Option<String>,
}

impl Chain {
    /// Load the model `name` and each supermodel in turn. `load` returns a
    /// model's MDL and MDX by name, or `None` when there is no such model;
    /// it is called once per model of the chain.
    ///
    /// A missing supermodel ends the chain and is named in
    /// [`missing`](Self::missing); a missing first model is
    /// [`Error::ModelNotFound`]; a chain that names a model twice is
    /// [`Error::SupermodelCycle`].
    pub fn load<E, F>(name: &str, mut load: F) -> std::result::Result<Chain, ChainError<E>>
    where
        F: FnMut(&str) -> std::result::Result<Option<(Vec<u8>, Vec<u8>)>, E>,
    {
        let mut files: Vec<ModelFiles> = Vec::new();
        let mut next = Some(name.to_owned());
        while let Some(n) = next.take() {
            if files.iter().any(|f| f.name.eq_ignore_ascii_case(&n)) {
                let mut chain: Vec<String> = files.into_iter().map(|f| f.name).collect();
                chain.push(n);
                return Err(Error::SupermodelCycle { chain }.into());
            }
            let found = load(&n).map_err(|error| ChainError::Load {
                name: n.clone(),
                error,
            })?;
            let Some((mdl, mdx)) = found else {
                if files.is_empty() {
                    return Err(Error::ModelNotFound { name: n }.into());
                }
                return Ok(Chain {
                    files,
                    missing: Some(n),
                });
            };
            next = supermodel_name(&mdl)?.map(str::to_owned);
            files.push(ModelFiles { name: n, mdl, mdx });
        }
        Ok(Chain {
            files,
            missing: None,
        })
    }

    /// Parse every model of the chain, the model first; each borrows from
    /// this chain's files. [`Lineage::new`] takes the result.
    pub fn parse(&self) -> Result<Vec<Model<'_>>> {
        self.files.iter().map(|f| parse(&f.mdl, &f.mdx)).collect()
    }
}

/// A model with its supermodels, the model first: what it inherits.
///
/// A model plays its own animations and, for a name it has none of, its
/// nearest supermodel's; an animation's nodes act on the model's nodes of
/// the same names. Names compare without case.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lineage<'m, 'a> {
    models: &'m [Model<'a>],
}

impl<'m, 'a> Lineage<'m, 'a> {
    /// The lineage of `models[0]`: each model must be followed by the
    /// supermodel it names (by header name, without case); the last may
    /// name one that is not given.
    pub fn new(models: &'m [Model<'a>]) -> Result<Self> {
        for pair in models.windows(2) {
            let (m, next) = (&pair[0], &pair[1]);
            if !m
                .supermodel
                .is_some_and(|s| s.eq_ignore_ascii_case(next.name))
            {
                return Err(Error::BadLineage {
                    model: m.name.to_owned(),
                    supermodel: m.supermodel.map(str::to_owned),
                    next: next.name.to_owned(),
                });
            }
        }
        Ok(Lineage { models })
    }

    /// The models, the model first.
    pub fn models(&self) -> &'m [Model<'a>] {
        self.models
    }

    /// The animation named `name` as the model plays it: its own, or its
    /// nearest supermodel's; with the index in [`models`](Self::models) of
    /// the model it comes from.
    pub fn animation(&self, name: &str) -> Option<(usize, &'m Animation<'a>)> {
        self.models
            .iter()
            .enumerate()
            .find_map(|(i, m)| m.animation(name).map(|a| (i, a)))
    }

    /// Every animation the model plays, each name once: its own in stored
    /// order, then each supermodel's it does not override, nearest first;
    /// with the index of the model each comes from.
    pub fn animations(&self) -> Vec<(usize, &'m Animation<'a>)> {
        let mut out: Vec<(usize, &'m Animation<'a>)> = Vec::new();
        for (i, m) in self.models.iter().enumerate() {
            for a in &m.animations {
                if !out.iter().any(|(_, b)| b.name.eq_ignore_ascii_case(a.name)) {
                    out.push((i, a));
                }
            }
        }
        out
    }

    /// The node named `name` in the model, or else in its nearest
    /// supermodel that has one; with the index of the model.
    pub fn node(&self, name: &str) -> Option<(usize, &'m Node<'a>)> {
        self.models
            .iter()
            .enumerate()
            .find_map(|(i, m)| m.node(name).map(|n| (i, n)))
    }

    /// For each node of `animation`, the index in the model's own
    /// [`nodes`](Model::nodes) of the node it acts on: the model's root for
    /// the animation's root (which is named after the model the animation
    /// was made for, a supermodel's name in an inherited one), and the
    /// node of the same name for the others; `None` for a node the model
    /// lacks.
    pub fn bind(&self, animation: &Animation<'_>) -> Vec<Option<usize>> {
        let Some(model) = self.models.first() else {
            return vec![None; animation.nodes.len()];
        };
        animation
            .nodes
            .iter()
            .enumerate()
            .map(|(i, a)| {
                if i == 0 && !model.nodes.is_empty() {
                    return Some(0);
                }
                model
                    .nodes
                    .iter()
                    .position(|n| n.name.eq_ignore_ascii_case(a.name))
            })
            .collect()
    }
}
