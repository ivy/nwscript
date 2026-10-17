//! Which operand types each operator takes, and constant folding.
//!
//! The operator rules are the bytecode's: an operator applies to the type
//! pairs the NCS instruction set has an instruction for (`ADDIF` adds an int
//! to a float; there is no `GTIF`, so `1 < 2.0` does not check). No operand
//! is converted implicitly.

use crate::hir::{BinaryOp, Expr, ExprKind, UnaryOp};
use crate::types::{ConstValue, Type};

/// The result type of `lhs op rhs`, or `None` when the operator does not
/// apply to those types.
pub fn binary_type(op: BinaryOp, lhs: &Type, rhs: &Type) -> Option<Type> {
    use BinaryOp::*;
    use Type::{Float as F, Int as I, String as S, Vector as V};
    Some(match op {
        LogOr | LogAnd | BitOr | BitXor | BitAnd | Shl | Shr | UShr | Mod => match (lhs, rhs) {
            (I, I) => I,
            _ => return None,
        },
        Eq | Ne => {
            if lhs != rhs || matches!(lhs, Type::Void | Type::Action) {
                return None;
            }
            I
        }
        Lt | Gt | Le | Ge => match (lhs, rhs) {
            (I, I) | (F, F) => I,
            _ => return None,
        },
        Add => match (lhs, rhs) {
            (I, I) => I,
            (I, F) | (F, I) | (F, F) => F,
            (S, S) => S,
            (V, V) => V,
            _ => return None,
        },
        Sub => match (lhs, rhs) {
            (I, I) => I,
            (I, F) | (F, I) | (F, F) => F,
            (V, V) => V,
            _ => return None,
        },
        Mul => match (lhs, rhs) {
            (I, I) => I,
            (I, F) | (F, I) | (F, F) => F,
            (V, F) | (F, V) => V,
            _ => return None,
        },
        Div => match (lhs, rhs) {
            (I, I) => I,
            (I, F) | (F, I) | (F, F) => F,
            (V, F) => V,
            _ => return None,
        },
    })
}

/// The result type of a prefix operator, or `None`.
pub fn unary_type(op: UnaryOp, operand: &Type) -> Option<Type> {
    match (op, operand) {
        (UnaryOp::Neg | UnaryOp::Plus, Type::Int | Type::Float) => Some(operand.clone()),
        (UnaryOp::Not | UnaryOp::BitNot, Type::Int) => Some(Type::Int),
        _ => None,
    }
}

/// The value of an expression the language reads as a constant: a literal,
/// a `nwscript.nss` constant, either negated, or a vector of float
/// constants. This is what a parameter's default and a `case` label may be;
/// a script global is a variable, never a constant.
pub fn fold(e: &Expr) -> Option<ConstValue> {
    match &e.kind {
        ExprKind::Literal(v) | ExprKind::Constant { value: v, .. } => Some(v.clone()),
        ExprKind::Unary {
            op: UnaryOp::Neg,
            operand,
        } => match fold(operand)? {
            ConstValue::Int(i) => Some(ConstValue::Int(i.wrapping_neg())),
            ConstValue::Float(f) => Some(ConstValue::Float(-f)),
            _ => None,
        },
        ExprKind::Vector(parts) => {
            let mut out = [0.0; 3];
            for (o, p) in out.iter_mut().zip(parts.iter()) {
                match fold(p)? {
                    ConstValue::Float(f) => *o = f,
                    _ => return None,
                }
            }
            Some(ConstValue::Vector(out))
        }
        _ => None,
    }
}
