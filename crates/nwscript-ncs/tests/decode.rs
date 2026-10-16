//! Decoding hand-written byte sequences, and a property round-trip.

use nwscript_diag::{FileId, SourceMap};
use nwscript_ncs::{
    Instruction, Layout, Opcode, Operands, Program, Type, decode, decode_named, encode,
};
use proptest::prelude::*;

const FILE: FileId = FileId(7);

/// A file of these instruction bytes, with a correct size record.
fn file(code: &[u8]) -> Vec<u8> {
    let mut out = b"NCS V1.0\x42".to_vec();
    out.extend_from_slice(&((13 + code.len()) as u32).to_be_bytes());
    out.extend_from_slice(code);
    out
}

/// `void main() { int i = 3; }`'s shape: JSR main, RETN, then main.
const SMALL: &[u8] = &[
    0x1E, 0x00, 0x00, 0x00, 0x00, 0x08, // JSR +8
    0x20, 0x00, // RETN
    0x04, 0x03, 0x00, 0x00, 0x00, 0x03, // CONSTI 3
    0x04, 0x05, 0x00, 0x02, b'h', b'i', // CONSTS "hi"
    0x0B, 0x24, 0x00, 0x0C, // EQUALTT 12
    0x2C, 0x10, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x08, // STORE_STATE 4, 8
    0x21, 0x01, 0x00, 0x0C, 0x00, 0x04, 0x00, 0x04, // DESTRUCT 12, 4, 4
    0x1B, 0x00, 0xFF, 0xFF, 0xFF, 0xFC, // MOVSP -4
    0x20, 0x00, // RETN
];

#[test]
fn decodes_operands_and_offsets() {
    let bytes = file(SMALL);
    let p = decode(FILE, &bytes).unwrap();
    let text: Vec<String> = p.instructions().iter().map(|i| i.to_string()).collect();
    assert_eq!(
        text,
        [
            "JSR 8",
            "RETN",
            "CONSTI 3",
            "CONSTS \"hi\"",
            "EQUALTT 12",
            "STORE_STATE 4, 8",
            "DESTRUCT 12, 4, 4",
            "MOVSP -4",
            "RETN",
        ]
    );
    assert_eq!(p.offsets(), [13, 19, 21, 27, 33, 37, 47, 55, 61]);
    assert_eq!(p.instructions()[0].jump_target(13), Some(21));
    assert_eq!(p.index_at(21), Some(2));
    assert_eq!(p.index_at(22), None);
    assert_eq!(p.file_len() as usize, bytes.len());
    assert_eq!(encode(&p), bytes);
    assert_eq!(Program::new(p.instructions().to_vec()), p);
}

#[test]
fn empty_program() {
    let bytes = file(&[]);
    let p = decode(FILE, &bytes).unwrap();
    assert!(p.is_empty());
    assert_eq!(encode(&p), bytes);
}

fn err(bytes: &[u8]) -> (String, String, usize) {
    let d = decode(FILE, bytes).unwrap_err();
    let span = d.primary_span().expect("a primary span");
    assert_eq!(span.file, FILE);
    (d.code.id.to_string(), d.message, span.start)
}

#[test]
fn truncated_inside_an_instruction_names_its_offset() {
    let mut bytes = file(SMALL);
    // Cut inside CONSTI at offset 21, leaving the size record as it was.
    bytes.truncate(24);
    let (code, msg, at) = err(&bytes);
    assert_eq!(code, "E0105");
    assert_eq!(at, 21);
    assert!(msg.contains("offset 21"), "{msg}");
    assert!(msg.contains("offset 24"), "{msg}");
}

#[test]
fn truncated_between_instructions_names_the_end() {
    let mut bytes = file(SMALL);
    bytes.truncate(21);
    let (code, msg, at) = err(&bytes);
    assert_eq!(code, "E0105");
    assert_eq!(at, 21);
    assert!(msg.contains("offset 21"), "{msg}");
}

#[test]
fn truncated_string_length() {
    let bytes = file(&[0x04, 0x05, 0x00]);
    let (code, _, at) = err(&bytes);
    assert_eq!((code.as_str(), at), ("E0105", 13));
    let bytes = file(&[0x04, 0x05, 0x00, 0x05, b'a']);
    let (code, msg, at) = err(&bytes);
    assert_eq!((code.as_str(), at), ("E0105", 13));
    assert!(msg.contains("needs 9 bytes"), "{msg}");
}

#[test]
fn truncated_header() {
    let (code, msg, at) = err(b"NCS V1.0\x42\x00");
    assert_eq!((code.as_str(), at), ("E0105", 10));
    assert!(msg.contains("offset 10"), "{msg}");
    let (code, _, at) = err(b"NCS");
    assert_eq!((code.as_str(), at), ("E0105", 3));
}

#[test]
fn not_ncs() {
    let (code, _, at) = err(b"// a comment, not bytecode");
    assert_eq!((code.as_str(), at), ("E0101", 0));
    let (code, _, _) = err(b"");
    assert_eq!(code, "E0105");
}

#[test]
fn size_record() {
    let mut bytes = file(SMALL);
    bytes[8] = 0x43;
    let (code, _, at) = err(&bytes);
    assert_eq!((code.as_str(), at), ("E0102", 8));

    let mut bytes = file(SMALL);
    bytes.extend_from_slice(&[0x20, 0x00]);
    let (code, msg, at) = err(&bytes);
    assert_eq!((code.as_str(), at), ("E0102", 63));
    assert!(
        msg.contains("declares 63 bytes but the file has 65"),
        "{msg}"
    );
}

#[test]
fn unknown_opcode() {
    let (code, msg, at) = err(&file(&[0x20, 0x00, 0x2E, 0x00]));
    assert_eq!((code.as_str(), at), ("E0103", 15));
    assert!(msg.contains("0x2E"), "{msg}");
    let (code, _, at) = err(&file(&[0x00, 0x00]));
    assert_eq!((code.as_str(), at), ("E0103", 13));
}

#[test]
fn wrong_type() {
    // GTSS is not an instruction.
    let (code, msg, at) = err(&file(&[0x0E, 0x23]));
    assert_eq!((code.as_str(), at), ("E0104", 14));
    assert!(msg.contains("GT") && msg.contains("0x23"), "{msg}");
    // STORE_STATE takes only its documented resume offset.
    let (code, _, _) = err(&file(&[0x2C, 0x11, 0, 0, 0, 0, 0, 0, 0, 0]));
    assert_eq!(code, "E0104");
}

#[test]
fn decode_named_registers_the_file() {
    let mut sources = SourceMap::default();
    let d = decode_named("cut.ncs", b"NCS V1.0\x42", &mut sources).unwrap_err();
    let span = d.primary_span().unwrap();
    assert_eq!(sources.get(span.file).name, "cut.ncs");
}

/// Every type byte an opcode accepts.
fn types_of(op: Opcode) -> Vec<Type> {
    (0..=255u8).map(Type).filter(|t| op.accepts(*t)).collect()
}

fn operands(layout: Layout) -> BoxedStrategy<Operands> {
    match layout {
        Layout::None => Just(Operands::None).boxed(),
        Layout::Stack => (any::<i32>(), any::<u16>())
            .prop_map(|(offset, size)| Operands::Stack { offset, size })
            .boxed(),
        Layout::Int => any::<i32>().prop_map(Operands::Int).boxed(),
        Layout::Float => any::<u32>()
            .prop_map(|b| Operands::Float(f32::from_bits(b)))
            .boxed(),
        Layout::String => proptest::collection::vec(any::<u8>(), 0..40)
            .prop_map(Operands::String)
            .boxed(),
        Layout::Object => any::<i32>().prop_map(Operands::Object).boxed(),
        Layout::Action => (any::<u16>(), any::<u8>())
            .prop_map(|(routine, argc)| Operands::Action { routine, argc })
            .boxed(),
        Layout::Size => any::<u16>().prop_map(Operands::Size).boxed(),
        Layout::Offset => any::<i32>().prop_map(Operands::Offset).boxed(),
        Layout::Jump => any::<i32>().prop_map(Operands::Jump).boxed(),
        Layout::Destruct => (any::<u16>(), any::<u16>(), any::<u16>())
            .prop_map(|(size, keep_offset, keep_size)| Operands::Destruct {
                size,
                keep_offset,
                keep_size,
            })
            .boxed(),
        Layout::StoreState => (any::<u32>(), any::<u32>())
            .prop_map(|(globals, locals)| Operands::StoreState { globals, locals })
            .boxed(),
    }
}

fn instruction() -> impl Strategy<Value = Instruction> {
    let forms: Vec<(Opcode, Type)> = Opcode::ALL
        .iter()
        .flat_map(|&op| types_of(op).into_iter().map(move |t| (op, t)))
        .collect();
    proptest::sample::select(forms).prop_flat_map(|(op, ty)| {
        operands(op.layout(ty).unwrap()).prop_map(move |o| Instruction::new(op, ty, o))
    })
}

#[test]
fn every_opcode_has_a_form() {
    for &op in Opcode::ALL {
        assert!(!types_of(op).is_empty(), "{op} accepts no type");
    }
}

proptest! {
    #[test]
    fn roundtrip_generated(instructions in proptest::collection::vec(instruction(), 0..64)) {
        prop_assert!(instructions.iter().all(Instruction::is_valid));
        let program = Program::new(instructions);
        let bytes = encode(&program);
        prop_assert_eq!(bytes.len(), program.file_len() as usize);
        let back = decode(FILE, &bytes).map_err(|d| TestCaseError::fail(d.message))?;
        prop_assert_eq!(back.offsets(), program.offsets());
        prop_assert_eq!(encode(&back), bytes);
        // Structural equality, with floats compared by bit pattern.
        for (a, b) in back.instructions().iter().zip(program.instructions()) {
            match (&a.operands, &b.operands) {
                (Operands::Float(x), Operands::Float(y)) => prop_assert_eq!(x.to_bits(), y.to_bits()),
                _ => prop_assert_eq!(a, b),
            }
        }
    }

    #[test]
    fn decode_never_panics(tail in proptest::collection::vec(any::<u8>(), 0..64)) {
        let mut bytes = file(&tail);
        let _ = decode(FILE, &bytes);
        bytes.truncate(bytes.len() / 2);
        let _ = decode(FILE, &bytes);
    }
}
