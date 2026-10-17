//! A list of instructions with symbolic jump targets, laid out into a
//! program once every label is placed.

use nwscript_ncs::{Instruction, Opcode, Operands, Program, Type};

/// A position in the code, placed once.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Label(pub u32);

enum Item {
    Ins(Instruction),
    /// `JMP`, `JSR`, `JZ` or `JNZ` to a label.
    Jump(Opcode, Label),
    Label(Label),
}

#[derive(Default)]
pub struct Emitter {
    items: Vec<Item>,
    labels: u32,
}

impl Emitter {
    pub fn label(&mut self) -> Label {
        self.labels += 1;
        Label(self.labels - 1)
    }

    pub fn place(&mut self, l: Label) {
        self.items.push(Item::Label(l));
    }

    pub fn ins(&mut self, opcode: Opcode, ty: Type, operands: Operands) {
        self.items
            .push(Item::Ins(Instruction::new(opcode, ty, operands)));
    }

    pub fn jump(&mut self, opcode: Opcode, to: Label) {
        self.items.push(Item::Jump(opcode, to));
    }

    /// Resolve the labels. Every label jumped to must have been placed.
    pub fn finish(self) -> Program {
        let mut at = vec![u32::MAX; self.labels as usize];
        let mut offset = nwscript_ncs::CODE_START;
        for item in &self.items {
            match item {
                Item::Ins(i) => offset += i.len() as u32,
                Item::Jump(..) => offset += 6,
                Item::Label(l) => at[l.0 as usize] = offset,
            }
        }
        let mut out = Vec::with_capacity(self.items.len());
        let mut offset = nwscript_ncs::CODE_START;
        for item in self.items {
            match item {
                Item::Ins(i) => {
                    offset += i.len() as u32;
                    out.push(i);
                }
                Item::Jump(op, l) => {
                    let target = at[l.0 as usize];
                    assert_ne!(target, u32::MAX, "a jump to a label never placed");
                    out.push(Instruction::new(
                        op,
                        Type::VOID,
                        Operands::Jump(target as i32 - offset as i32),
                    ));
                    offset += 6;
                }
                Item::Label(_) => {}
            }
        }
        Program::new(out)
    }
}
