//! The two bytes that start every instruction: the opcode and its type qualifier.

use std::fmt;

macro_rules! opcodes {
    ($( $(#[$m:meta])* $variant:ident = $byte:literal, $name:literal; )*) => {
        /// An instruction's first byte.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Opcode {
            $( $(#[$m])* $variant = $byte, )*
        }

        impl Opcode {
            /// Every opcode, in byte order.
            pub const ALL: &[Opcode] = &[$(Opcode::$variant),*];

            /// The opcode with this byte, if there is one.
            pub fn from_byte(byte: u8) -> Option<Opcode> {
                match byte {
                    $( $byte => Some(Opcode::$variant), )*
                    _ => None,
                }
            }

            /// The mnemonic, without a type suffix.
            pub fn name(self) -> &'static str {
                match self {
                    $( Opcode::$variant => $name, )*
                }
            }
        }
    };
}

opcodes! {
    /// Copy `size` bytes from the top of the stack down to `offset` from SP.
    CpDownSp = 0x01, "CPDOWNSP";
    /// Reserve one stack element of the qualifier's type.
    RsAdd = 0x02, "RSADD";
    /// Copy `size` bytes at `offset` from SP to the top of the stack.
    CpTopSp = 0x03, "CPTOPSP";
    /// Push a constant of the qualifier's type.
    Const = 0x04, "CONST";
    /// Call engine routine `routine` with `argc` arguments.
    Action = 0x05, "ACTION";
    LogAnd = 0x06, "LOGAND";
    LogOr = 0x07, "LOGOR";
    IncOr = 0x08, "INCOR";
    ExcOr = 0x09, "EXCOR";
    BoolAnd = 0x0A, "BOOLAND";
    Equal = 0x0B, "EQUAL";
    NEqual = 0x0C, "NEQUAL";
    Geq = 0x0D, "GEQ";
    Gt = 0x0E, "GT";
    Lt = 0x0F, "LT";
    Leq = 0x10, "LEQ";
    ShLeft = 0x11, "SHLEFT";
    ShRight = 0x12, "SHRIGHT";
    UShRight = 0x13, "USHRIGHT";
    Add = 0x14, "ADD";
    Sub = 0x15, "SUB";
    Mul = 0x16, "MUL";
    Div = 0x17, "DIV";
    Mod = 0x18, "MOD";
    Neg = 0x19, "NEG";
    Comp = 0x1A, "COMP";
    /// Adjust SP by `offset` bytes.
    MovSp = 0x1B, "MOVSP";
    /// Save the whole stack for a deferred action; its qualifier is the resume offset.
    StoreStateAll = 0x1C, "STORE_STATEALL";
    Jmp = 0x1D, "JMP";
    Jsr = 0x1E, "JSR";
    Jz = 0x1F, "JZ";
    Retn = 0x20, "RETN";
    /// Remove `size` bytes from the top of the stack, keeping `keep_size` bytes at `keep_offset`.
    Destruct = 0x21, "DESTRUCT";
    Not = 0x22, "NOT";
    DecISp = 0x23, "DECISP";
    IncISp = 0x24, "INCISP";
    Jnz = 0x25, "JNZ";
    CpDownBp = 0x26, "CPDOWNBP";
    CpTopBp = 0x27, "CPTOPBP";
    DecIBp = 0x28, "DECIBP";
    IncIBp = 0x29, "INCIBP";
    SaveBp = 0x2A, "SAVEBP";
    RestoreBp = 0x2B, "RESTOREBP";
    /// Save `globals` bytes below BP and `locals` bytes below SP for a deferred action; its qualifier is the resume offset.
    StoreState = 0x2C, "STORE_STATE";
    Nop = 0x2D, "NOP";
}

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// An instruction's second byte, kept as the byte it is.
///
/// For most opcodes it names the operand types (`INT`, `FLOAT_INT`, ...).
/// For [`Opcode::StoreState`] and [`Opcode::StoreStateAll`] it is the byte
/// distance from the instruction to the code the saved state resumes at.
/// Which bytes an opcode accepts is [`Opcode::accepts`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Type(pub u8);

impl Type {
    /// No operand types: control flow, `ACTION`, `MOVSP`, the BP saves.
    pub const VOID: Type = Type(0x00);
    /// A stack-addressed copy or destruct.
    pub const STACK: Type = Type(0x01);
    pub const INT: Type = Type(0x03);
    pub const FLOAT: Type = Type(0x04);
    pub const STRING: Type = Type(0x05);
    pub const OBJECT: Type = Type(0x06);
    /// The first engine type; engine type `n` is `ENGINE_0 + n`, `n` in 0..16.
    pub const ENGINE_0: Type = Type(0x10);
    pub const INT_INT: Type = Type(0x20);
    pub const FLOAT_FLOAT: Type = Type(0x21);
    pub const OBJECT_OBJECT: Type = Type(0x22);
    pub const STRING_STRING: Type = Type(0x23);
    pub const STRUCT_STRUCT: Type = Type(0x24);
    pub const INT_FLOAT: Type = Type(0x25);
    pub const FLOAT_INT: Type = Type(0x26);
    /// The first engine-type pair; pair `n` is `ENGINE_ENGINE_0 + n`, `n` in 0..10.
    pub const ENGINE_ENGINE_0: Type = Type(0x30);
    pub const VECTOR_VECTOR: Type = Type(0x3A);
    pub const VECTOR_FLOAT: Type = Type(0x3B);
    pub const FLOAT_VECTOR: Type = Type(0x3C);
    /// `STORE_STATE`'s resume offset: its own 10 bytes and a 6-byte `JMP`.
    pub const STORE_STATE: Type = Type(0x10);
    /// `STORE_STATEALL`'s resume offset: its own 2 bytes and a 6-byte `JMP`.
    pub const STORE_STATE_ALL: Type = Type(0x08);

    /// The engine type index, when this is a unary engine type.
    pub fn engine(self) -> Option<u8> {
        (0x10..=0x1F).contains(&self.0).then(|| self.0 - 0x10)
    }

    /// The engine type index, when this is a pair of one engine type.
    pub fn engine_pair(self) -> Option<u8> {
        (0x30..=0x39).contains(&self.0).then(|| self.0 - 0x30)
    }

    /// The conventional mnemonic suffix: `I`, `FF`, `E3`, `EE3`, `TT`, ...;
    /// empty for `VOID` and `STACK`.
    pub fn suffix(self) -> String {
        if let Some(n) = self.engine() {
            return format!("E{n}");
        }
        if let Some(n) = self.engine_pair() {
            return format!("EE{n}");
        }
        match self {
            Type::VOID | Type::STACK => "",
            Type::INT => "I",
            Type::FLOAT => "F",
            Type::STRING => "S",
            Type::OBJECT => "O",
            Type::INT_INT => "II",
            Type::FLOAT_FLOAT => "FF",
            Type::OBJECT_OBJECT => "OO",
            Type::STRING_STRING => "SS",
            Type::STRUCT_STRUCT => "TT",
            Type::INT_FLOAT => "IF",
            Type::FLOAT_INT => "FI",
            Type::VECTOR_VECTOR => "VV",
            Type::VECTOR_FLOAT => "VF",
            Type::FLOAT_VECTOR => "FV",
            _ => return format!("{:02X}", self.0),
        }
        .to_string()
    }
}

/// The byte layout of an instruction's operands after its two leading bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Layout {
    /// No operands.
    None,
    /// `offset: i32, size: u16`.
    Stack,
    /// One `i32`.
    Int,
    /// One IEEE-754 `f32`.
    Float,
    /// `len: u16`, then `len` bytes.
    String,
    /// One `i32` object id.
    Object,
    /// `routine: u16, argc: u8`.
    Action,
    /// One `u16` struct size.
    Size,
    /// One `i32` SP- or BP-relative byte offset.
    Offset,
    /// One `i32` byte offset from the start of the instruction.
    Jump,
    /// `size: u16, keep_offset: u16, keep_size: u16`.
    Destruct,
    /// `globals: u32, locals: u32`.
    StoreState,
}

impl Layout {
    /// The operands' encoded length, or `None` when it depends on the bytes (a string).
    pub fn fixed_len(self) -> Option<usize> {
        Some(match self {
            Layout::None => 0,
            Layout::Stack => 6,
            Layout::Int | Layout::Float | Layout::Object | Layout::Offset | Layout::Jump => 4,
            Layout::String => return None,
            Layout::Action => 3,
            Layout::Size => 2,
            Layout::Destruct => 6,
            Layout::StoreState => 8,
        })
    }
}

impl Opcode {
    /// The operand layout of this opcode with this type byte, or `None`
    /// when the pair is not a documented instruction.
    pub fn layout(self, ty: Type) -> Option<Layout> {
        use Opcode::*;
        let unary_value = |t: Type| {
            matches!(t, Type::INT | Type::FLOAT | Type::STRING | Type::OBJECT)
                || t.engine().is_some()
        };
        let comparable = |t: Type| {
            matches!(
                t,
                Type::INT_INT | Type::FLOAT_FLOAT | Type::OBJECT_OBJECT | Type::STRING_STRING
            ) || t.engine_pair().is_some()
        };
        let ok = |cond: bool, layout: Layout| cond.then_some(layout);
        match self {
            CpDownSp | CpTopSp | CpDownBp | CpTopBp => ok(ty == Type::STACK, Layout::Stack),
            RsAdd => ok(unary_value(ty), Layout::None),
            Const => match ty {
                Type::INT => Some(Layout::Int),
                Type::FLOAT => Some(Layout::Float),
                Type::STRING => Some(Layout::String),
                Type::OBJECT => Some(Layout::Object),
                _ => None,
            },
            Action => ok(ty == Type::VOID, Layout::Action),
            LogAnd | LogOr | IncOr | ExcOr | BoolAnd | ShLeft | ShRight | UShRight | Mod => {
                ok(ty == Type::INT_INT, Layout::None)
            }
            Equal | NEqual => match ty {
                Type::STRUCT_STRUCT => Some(Layout::Size),
                t => ok(comparable(t), Layout::None),
            },
            Geq | Gt | Lt | Leq => ok(
                matches!(ty, Type::INT_INT | Type::FLOAT_FLOAT),
                Layout::None,
            ),
            Add => ok(
                matches!(
                    ty,
                    Type::INT_INT
                        | Type::INT_FLOAT
                        | Type::FLOAT_INT
                        | Type::FLOAT_FLOAT
                        | Type::STRING_STRING
                        | Type::VECTOR_VECTOR
                ),
                Layout::None,
            ),
            Sub => ok(
                matches!(
                    ty,
                    Type::INT_INT
                        | Type::INT_FLOAT
                        | Type::FLOAT_INT
                        | Type::FLOAT_FLOAT
                        | Type::VECTOR_VECTOR
                ),
                Layout::None,
            ),
            Mul => ok(
                matches!(
                    ty,
                    Type::INT_INT
                        | Type::INT_FLOAT
                        | Type::FLOAT_INT
                        | Type::FLOAT_FLOAT
                        | Type::VECTOR_FLOAT
                        | Type::FLOAT_VECTOR
                ),
                Layout::None,
            ),
            Div => ok(
                matches!(
                    ty,
                    Type::INT_INT
                        | Type::INT_FLOAT
                        | Type::FLOAT_INT
                        | Type::FLOAT_FLOAT
                        | Type::VECTOR_FLOAT
                ),
                Layout::None,
            ),
            Neg => ok(matches!(ty, Type::INT | Type::FLOAT), Layout::None),
            Comp | Not => ok(ty == Type::INT, Layout::None),
            MovSp => ok(ty == Type::VOID, Layout::Offset),
            DecISp | IncISp | DecIBp | IncIBp => ok(ty == Type::INT, Layout::Offset),
            Jmp | Jsr | Jz | Jnz => ok(ty == Type::VOID, Layout::Jump),
            Retn | SaveBp | RestoreBp | Nop => ok(ty == Type::VOID, Layout::None),
            Destruct => ok(ty == Type::STACK, Layout::Destruct),
            // The qualifier is the resume offset: past this instruction and the `JMP` that follows it.
            StoreState => ok(ty == Type::STORE_STATE, Layout::StoreState),
            StoreStateAll => ok(ty == Type::STORE_STATE_ALL, Layout::None),
        }
    }

    /// True when this opcode with this type byte is a documented instruction.
    pub fn accepts(self, ty: Type) -> bool {
        self.layout(ty).is_some()
    }

    /// The mnemonic with the type suffix the type byte selects: `RSADDI`,
    /// `EQUALTT`, `NEGF`; the bare name where the type byte is fixed
    /// (`DECISP`, `STORE_STATE`).
    pub fn mnemonic(self, ty: Type) -> String {
        use Opcode::*;
        match self {
            StoreState | StoreStateAll | DecISp | IncISp | DecIBp | IncIBp => self.name().into(),
            _ => format!("{}{}", self.name(), ty.suffix()),
        }
    }

    /// True for the jumps whose operand is relative to the instruction: `JMP`, `JSR`, `JZ`, `JNZ`.
    pub fn is_jump(self) -> bool {
        matches!(self, Opcode::Jmp | Opcode::Jsr | Opcode::Jz | Opcode::Jnz)
    }
}
