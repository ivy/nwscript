//! Print every NCS mnemonic grouped by opcode, for `docs/asm.md`.
fn main() {
    use nwscript_ncs::{Opcode, Type};
    for &op in Opcode::ALL {
        let forms: Vec<String> = (0..=255u8)
            .filter(|&b| op.accepts(Type(b)))
            .map(|b| format!("`{}`", op.mnemonic(Type(b))))
            .collect();
        println!(
            "| `{:#04x}` {} | {} |",
            op as u8,
            op.name(),
            forms.join(" ")
        );
    }
}
