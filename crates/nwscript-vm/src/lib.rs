//! The NCS interpreter: stack, frames, STORE_STATE closures, an engine
//! trait and a recording engine.
//!
//! A script touches the game only through engine routines (`ACTION`), so
//! running its bytecode against an [`Engine`] that records each call is a
//! complete test double. The machine executes every instruction as the
//! engine's virtual machine does; what the engine routines do is the
//! [`Engine`]'s business, and [`Recording`] does nothing but answer with a
//! default and say so.

mod machine;
mod value;

pub use machine::{Limits, run};
pub use value::{Closure, Ty, Value};

use std::fmt;

/// An engine routine's signature: the argument types in declaration order,
/// first parameter first, and the return type. Its position in the table is
/// its `ACTION` number.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Routine {
    pub name: String,
    pub params: Vec<Ty>,
    pub ret: Ty,
}

/// What an engine routine returned, and whether anything modelled it.
#[derive(Clone, Debug, PartialEq)]
pub struct Reply {
    pub value: Value,
    /// False when the value is a default standing in for behaviour no model covers.
    pub modelled: bool,
}

/// The engine behind `ACTION`: given the routine and its arguments, the value
/// it returns.
pub trait Engine {
    fn call(&mut self, routine: u16, sig: &Routine, args: &[Value]) -> Reply;
}

/// An engine that models nothing: every routine returns its type's default
/// and the call is marked unmodelled. Defaults are 0, 0.0, "", `OBJECT_INVALID`,
/// a null engine structure, and the zero vector.
#[derive(Debug, Default)]
pub struct Recording;

impl Engine for Recording {
    fn call(&mut self, _routine: u16, sig: &Routine, _args: &[Value]) -> Reply {
        Reply {
            value: Value::default_of(&sig.ret),
            modelled: false,
        }
    }
}

/// One engine call as the script made it.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    pub routine: u16,
    pub name: String,
    pub args: Vec<Value>,
    pub ret: Value,
    pub modelled: bool,
    /// The deferred action this call ran in, numbered in the order the
    /// script created them; `None` for the script's own body.
    pub deferred: Option<usize>,
}

impl fmt::Display for Call {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(d) = self.deferred {
            write!(f, "[deferred {d}] ")?;
        }
        write!(f, "{}(", self.name)?;
        for (i, a) in self.args.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{a}")?;
        }
        write!(f, ")")?;
        if self.ret != Value::Void {
            write!(f, " -> {}", self.ret)?;
        }
        if !self.modelled {
            f.write_str(" (unmodelled)")?;
        }
        Ok(())
    }
}

/// Why a run stopped before finishing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fault {
    /// The file offset of the instruction that faulted.
    pub offset: u32,
    pub kind: FaultKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FaultKind {
    /// The stack held too few bytes for the instruction.
    StackUnderflow,
    /// A stack element was not of the type the instruction reads.
    TypeMismatch {
        expected: &'static str,
        found: &'static str,
    },
    /// An access below the bottom of the stack or misaligned to four bytes.
    BadStackOffset(i32),
    /// A jump to an offset where no instruction starts.
    BadJump(u32),
    DivisionByZero,
    /// An `ACTION` naming a routine the table does not have.
    UnknownRoutine(u16),
    /// The stack does not hold the arguments the routine's signature calls
    /// for: the script was compiled against different declarations.
    ArgumentMismatch {
        routine: u16,
        expected: &'static str,
        found: &'static str,
    },
    /// An `ACTION` whose action parameter has no preceding `STORE_STATE`.
    NoStoredState,
    /// The instruction budget ran out; the script may loop on engine state.
    Budget,
    /// An instruction the machine does not execute (`STORE_STATEALL`).
    Unsupported(&'static str),
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at offset {}: ", self.offset)?;
        match &self.kind {
            FaultKind::StackUnderflow => f.write_str("stack underflow"),
            FaultKind::TypeMismatch { expected, found } => {
                write!(f, "expected {expected} on the stack, found {found}")
            }
            FaultKind::BadStackOffset(o) => {
                write!(f, "stack offset {o} is out of range or misaligned")
            }
            FaultKind::BadJump(t) => write!(f, "jump to {t}, where no instruction starts"),
            FaultKind::DivisionByZero => f.write_str("division by zero"),
            FaultKind::UnknownRoutine(r) => write!(f, "engine routine {r} is not in the table"),
            FaultKind::ArgumentMismatch {
                routine,
                expected,
                found,
            } => {
                write!(
                    f,
                    "routine {routine} takes {expected} where the stack holds {found}; the script was compiled against a different nwscript.nss"
                )
            }
            FaultKind::NoStoredState => {
                f.write_str("an action argument with no STORE_STATE before it")
            }
            FaultKind::Budget => f.write_str("instruction budget exhausted"),
            FaultKind::Unsupported(what) => write!(f, "{what} is not supported"),
        }
    }
}

/// The outcome of running a script and its deferred actions.
#[derive(Clone, Debug, PartialEq)]
pub struct Run {
    /// The value the script's entry returned: `StartingConditional`'s int,
    /// or `None` for `main`.
    pub result: Option<Value>,
    pub trace: Vec<Call>,
    /// Routines whose replies no model produced, by name, sorted.
    pub unmodelled: Vec<String>,
    pub fault: Option<Fault>,
    /// Instructions executed, deferred actions included.
    pub steps: u64,
    /// Deferred actions the script created that were not run (over the limit).
    pub deferred_skipped: usize,
    /// The last instructions executed, when [`Limits::history`] asks for them.
    pub history: Vec<String>,
}

impl FaultKind {
    /// True when the fault comes from what the engine answered or what the
    /// script was compiled against, not from the machine: a division by a
    /// defaulted zero, a loop on engine state, arguments for other declarations.
    pub fn is_engine_dependent(&self) -> bool {
        matches!(
            self,
            FaultKind::DivisionByZero | FaultKind::Budget | FaultKind::ArgumentMismatch { .. }
        )
    }
}
