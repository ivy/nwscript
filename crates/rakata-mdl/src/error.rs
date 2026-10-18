//! The errors this crate reports, each with a registered diagnostic code.

use std::fmt;

/// Diagnostic codes for models: the `RE24xx` range of the `rakata`
/// registry. Each is a stable string a caller may match on and a user may
/// look up.
// Registered, with explanations, in `crates/rakata/src/codes.rs`; this crate
// carries only the strings.
pub mod codes {
    /// The MDL does not begin with the binary model's zero word.
    pub const NOT_BINARY: &str = "RE2401";
    /// A structure of the MDL lies past the end of its model data.
    pub const TRUNCATED: &str = "RE2402";
    /// A mesh's vertices lie past the end of the MDX.
    pub const MDX_OUT_OF_RANGE: &str = "RE2403";
    /// The node tree reaches a node twice, or a node names no name.
    pub const BAD_TREE: &str = "RE2404";
    /// A name or texture field that is not ASCII ended by a NUL.
    pub const BAD_NAME: &str = "RE2405";
    /// A mesh's vertex layout that does not fit its stride.
    pub const BAD_VERTEX_LAYOUT: &str = "RE2406";
    /// A face or index naming a vertex the mesh does not have.
    pub const BAD_INDEX: &str = "RE2407";
    /// A controller whose keys lie past its node's controller data.
    pub const BAD_CONTROLLER: &str = "RE2408";
    /// A skin whose bone map, qbones and tbones disagree.
    pub const BAD_SKIN: &str = "RE2409";
    /// A supermodel chain that comes back to a model it has passed.
    pub const SUPERMODEL_CYCLE: &str = "RE2410";
    /// The loader failed to read a model of a supermodel chain.
    pub const SUPERMODEL_LOAD: &str = "RE2411";
    /// The model a supermodel chain starts from is not found.
    pub const MODEL_NOT_FOUND: &str = "RE2412";
    /// Models given as a chain whose next is not the supermodel named.
    pub const BAD_LINEAGE: &str = "RE2413";
}

/// An error in reading a model.
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    /// The first word of the MDL is not zero, so it is not a binary model
    /// (an ASCII model begins with text).
    NotBinary {
        /// The first four bytes, little-endian.
        first: u32,
    },
    /// A structure lies past the end of the model data.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// Its offset from the start of the model data (the file offset
        /// less 12).
        offset: usize,
        /// The length it needs.
        needed: usize,
        /// The length of the model data.
        available: usize,
    },
    /// A mesh's vertex block lies past the end of the MDX.
    MdxOutOfRange {
        /// The mesh's node.
        node: String,
        /// The block's offset in the MDX.
        offset: usize,
        /// The block's length.
        needed: usize,
        /// The MDX's length.
        available: usize,
    },
    /// The node tree is not a tree, or a node's name index is out of range.
    BadTree {
        /// Why.
        reason: String,
    },
    /// A name that is not ASCII, or a fixed-width name with no NUL.
    BadName {
        /// Which name.
        what: &'static str,
        /// Its offset from the start of the model data.
        offset: usize,
    },
    /// A mesh's vertex layout that its stride cannot hold.
    BadVertexLayout {
        /// The mesh's node.
        node: String,
        /// Why.
        reason: String,
    },
    /// A face or an index list entry naming a vertex past the mesh's count.
    BadIndex {
        /// The mesh's node.
        node: String,
        /// The vertex index.
        index: u16,
        /// The mesh's vertex count.
        vertices: usize,
    },
    /// A controller whose time keys or values lie past the node's
    /// controller data.
    BadController {
        /// The node.
        node: String,
        /// The controller type.
        kind: u32,
        /// Why.
        reason: String,
    },
    /// A skin whose bone map does not name each local bone index once, or
    /// whose bone map, qbones and tbones differ in length.
    BadSkin {
        /// The skin's node.
        node: String,
        /// Why.
        reason: String,
    },
    /// A supermodel chain that names a model it has already passed.
    SupermodelCycle {
        /// The chain from its first model, ending with the name repeated.
        chain: Vec<String>,
    },
    /// The model a supermodel chain starts from is not found.
    ModelNotFound {
        /// Its name.
        name: String,
    },
    /// Models given as a supermodel chain whose next model is not the
    /// supermodel the one before names.
    BadLineage {
        /// The model whose supermodel is wrong or missing.
        model: String,
        /// The supermodel it names, if any.
        supermodel: Option<String>,
        /// The name of the model that follows it.
        next: String,
    },
}

/// This crate's result type.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            Error::NotBinary { .. } => codes::NOT_BINARY,
            Error::Truncated { .. } => codes::TRUNCATED,
            Error::MdxOutOfRange { .. } => codes::MDX_OUT_OF_RANGE,
            Error::BadTree { .. } => codes::BAD_TREE,
            Error::BadName { .. } => codes::BAD_NAME,
            Error::BadVertexLayout { .. } => codes::BAD_VERTEX_LAYOUT,
            Error::BadIndex { .. } => codes::BAD_INDEX,
            Error::BadController { .. } => codes::BAD_CONTROLLER,
            Error::BadSkin { .. } => codes::BAD_SKIN,
            Error::SupermodelCycle { .. } => codes::SUPERMODEL_CYCLE,
            Error::ModelNotFound { .. } => codes::MODEL_NOT_FOUND,
            Error::BadLineage { .. } => codes::BAD_LINEAGE,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: ", self.code())?;
        match self {
            Error::NotBinary { first } => write!(
                f,
                "not a binary model: the first word is {first:#010x}, not zero"
            ),
            Error::Truncated {
                what,
                offset,
                needed,
                available,
            } => write!(
                f,
                "the MDL's {what} at model offset {offset} needs {needed} bytes and the model data has {available}"
            ),
            Error::MdxOutOfRange {
                node,
                offset,
                needed,
                available,
            } => write!(
                f,
                "mesh `{node}`'s vertices at MDX offset {offset} need {needed} bytes and the MDX has {available}"
            ),
            Error::BadTree { reason } => write!(f, "node tree: {reason}"),
            Error::BadName { what, offset } => write!(
                f,
                "the {what} at model offset {offset} is not ASCII ended by a NUL"
            ),
            Error::BadVertexLayout { node, reason } => {
                write!(f, "mesh `{node}`'s vertex layout: {reason}")
            }
            Error::BadIndex {
                node,
                index,
                vertices,
            } => write!(
                f,
                "mesh `{node}` names vertex {index} and has {vertices} vertices"
            ),
            Error::BadController { node, kind, reason } => {
                write!(f, "node `{node}`'s controller of type {kind}: {reason}")
            }
            Error::BadSkin { node, reason } => write!(f, "skin `{node}`: {reason}"),
            Error::SupermodelCycle { chain } => write!(
                f,
                "the supermodel chain {} comes back to a model it has passed",
                chain.join(" -> ")
            ),
            Error::ModelNotFound { name } => write!(f, "model `{name}` is not found"),
            Error::BadLineage {
                model,
                supermodel,
                next,
            } => match supermodel {
                Some(s) => write!(
                    f,
                    "model `{model}` names supermodel `{s}` and is followed by `{next}`"
                ),
                None => write!(
                    f,
                    "model `{model}` has no supermodel and is followed by `{next}`"
                ),
            },
        }
    }
}

impl std::error::Error for Error {}

/// An error in loading a supermodel chain: the loader's own, or the
/// model's.
#[derive(Debug)]
pub enum ChainError<E> {
    /// The loader failed reading a model.
    Load {
        /// The model it was asked for.
        name: String,
        /// Its error.
        error: E,
    },
    /// A model of the chain is not readable, or the chain is not a chain.
    Model(Error),
}

impl<E> ChainError<E> {
    /// The registered diagnostic code for this error.
    pub fn code(&self) -> &'static str {
        match self {
            ChainError::Load { .. } => codes::SUPERMODEL_LOAD,
            ChainError::Model(e) => e.code(),
        }
    }
}

impl<E> From<Error> for ChainError<E> {
    fn from(e: Error) -> Self {
        ChainError::Model(e)
    }
}

impl<E: fmt::Display> fmt::Display for ChainError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChainError::Load { name, error } => write!(
                f,
                "{}: reading model `{name}` of a supermodel chain: {error}",
                self.code()
            ),
            ChainError::Model(e) => e.fmt(f),
        }
    }
}

impl<E: fmt::Debug + fmt::Display> std::error::Error for ChainError<E> {}
