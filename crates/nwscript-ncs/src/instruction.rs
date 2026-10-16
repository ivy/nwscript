//! One instruction: opcode, type byte, and operands, each kept as encoded.

use std::fmt;

use crate::opcode::{Layout, Opcode, Type};

/// An instruction's operands, typed by its [`Layout`].
///
/// Every field holds the exact encoded value, so encoding reproduces the
/// bytes: a float keeps its bit pattern, a string its raw (Windows-1252)
/// bytes.
#[derive(Clone, Debug, PartialEq)]
pub enum Operands {
    None,
    /// `CPDOWNSP`, `CPTOPSP`, `CPDOWNBP`, `CPTOPBP`.
    Stack {
        offset: i32,
        size: u16,
    },
    /// `CONSTI`.
    Int(i32),
    /// `CONSTF`.
    Float(f32),
    /// `CONSTS`: the string's bytes, at most `u16::MAX` of them.
    String(Vec<u8>),
    /// `CONSTO`: an object id.
    Object(i32),
    /// `ACTION`.
    Action {
        routine: u16,
        argc: u8,
    },
    /// `EQUALTT`, `NEQUALTT`: the struct's size in bytes.
    Size(u16),
    /// `MOVSP`, `DECISP`, `INCISP`, `DECIBP`, `INCIBP`.
    Offset(i32),
    /// `JMP`, `JSR`, `JZ`, `JNZ`: bytes from the start of this instruction.
    Jump(i32),
    /// `DESTRUCT`.
    Destruct {
        size: u16,
        keep_offset: u16,
        keep_size: u16,
    },
    /// `STORE_STATE`: bytes saved below BP and below SP.
    StoreState {
        globals: u32,
        locals: u32,
    },
}

impl Operands {
    /// The layout these operands encode with.
    pub fn layout(&self) -> Layout {
        match self {
            Operands::None => Layout::None,
            Operands::Stack { .. } => Layout::Stack,
            Operands::Int(_) => Layout::Int,
            Operands::Float(_) => Layout::Float,
            Operands::String(_) => Layout::String,
            Operands::Object(_) => Layout::Object,
            Operands::Action { .. } => Layout::Action,
            Operands::Size(_) => Layout::Size,
            Operands::Offset(_) => Layout::Offset,
            Operands::Jump(_) => Layout::Jump,
            Operands::Destruct { .. } => Layout::Destruct,
            Operands::StoreState { .. } => Layout::StoreState,
        }
    }
}

/// One decoded instruction.
///
/// [`Instruction::is_valid`] holds for everything [`crate::decode`] returns;
/// a hand-built instruction that breaks it still encodes, byte for byte, but
/// is not one the engine documents.
#[derive(Clone, Debug, PartialEq)]
pub struct Instruction {
    pub opcode: Opcode,
    pub ty: Type,
    pub operands: Operands,
}

impl Instruction {
    pub fn new(opcode: Opcode, ty: Type, operands: Operands) -> Self {
        Instruction {
            opcode,
            ty,
            operands,
        }
    }

    /// True when the opcode accepts the type byte and the operands have the layout the pair calls for.
    /// A `CONSTS` string longer than `u16::MAX` bytes is not valid.
    pub fn is_valid(&self) -> bool {
        let len_ok = match &self.operands {
            Operands::String(s) => s.len() <= usize::from(u16::MAX),
            _ => true,
        };
        len_ok && self.opcode.layout(self.ty) == Some(self.operands.layout())
    }

    /// The encoded length in bytes.
    pub fn len(&self) -> usize {
        2 + match &self.operands {
            Operands::String(s) => 2 + s.len(),
            other => other.layout().fixed_len().unwrap_or(0),
        }
    }

    /// Always false: an instruction is at least its two leading bytes.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// The absolute target of a jump at `offset`, or `None` when this is not a jump.
    pub fn jump_target(&self, offset: u32) -> Option<u32> {
        match self.operands {
            Operands::Jump(rel) if self.opcode.is_jump() => Some(offset.wrapping_add_signed(rel)),
            _ => None,
        }
    }

    /// Append the encoded bytes to `out`.
    ///
    /// A `CONSTS` string longer than `u16::MAX` bytes is truncated to its first `u16::MAX`.
    pub fn encode_into(&self, out: &mut Vec<u8>) {
        out.push(self.opcode as u8);
        out.push(self.ty.0);
        match &self.operands {
            Operands::None => {}
            Operands::Stack { offset, size } => {
                out.extend_from_slice(&offset.to_be_bytes());
                out.extend_from_slice(&size.to_be_bytes());
            }
            Operands::Int(v) | Operands::Object(v) | Operands::Offset(v) | Operands::Jump(v) => {
                out.extend_from_slice(&v.to_be_bytes());
            }
            Operands::Float(v) => out.extend_from_slice(&v.to_bits().to_be_bytes()),
            Operands::String(s) => {
                let s = &s[..s.len().min(usize::from(u16::MAX))];
                out.extend_from_slice(&(s.len() as u16).to_be_bytes());
                out.extend_from_slice(s);
            }
            Operands::Action { routine, argc } => {
                out.extend_from_slice(&routine.to_be_bytes());
                out.push(*argc);
            }
            Operands::Size(v) => out.extend_from_slice(&v.to_be_bytes()),
            Operands::Destruct {
                size,
                keep_offset,
                keep_size,
            } => {
                out.extend_from_slice(&size.to_be_bytes());
                out.extend_from_slice(&keep_offset.to_be_bytes());
                out.extend_from_slice(&keep_size.to_be_bytes());
            }
            Operands::StoreState { globals, locals } => {
                out.extend_from_slice(&globals.to_be_bytes());
                out.extend_from_slice(&locals.to_be_bytes());
            }
        }
    }
}

/// The mnemonic and operands, e.g. `CPDOWNSP -8, 4` or `CONSTS "hi"`.
impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.opcode.mnemonic(self.ty))?;
        match &self.operands {
            Operands::None => Ok(()),
            Operands::Stack { offset, size } => write!(f, " {offset}, {size}"),
            Operands::Int(v) | Operands::Object(v) | Operands::Offset(v) | Operands::Jump(v) => {
                write!(f, " {v}")
            }
            Operands::Float(v) => write!(f, " {v:?}"),
            Operands::String(s) => write!(f, " \"{}\"", s.escape_ascii()),
            Operands::Action { routine, argc } => write!(f, " {routine}, {argc}"),
            Operands::Size(v) => write!(f, " {v}"),
            Operands::Destruct {
                size,
                keep_offset,
                keep_size,
            } => write!(f, " {size}, {keep_offset}, {keep_size}"),
            Operands::StoreState { globals, locals } => write!(f, " {globals}, {locals}"),
        }
    }
}
