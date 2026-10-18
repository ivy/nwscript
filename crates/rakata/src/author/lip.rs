//! Lip-sync files made from a tool's mouth cues.

use rakata_lip::{Lip, Shape};

/// A track over a line `length` seconds long from cues, each the time a
/// shape begins: the cues in time order and a rest at `length`, so the
/// mouth closes as the line ends ([`Lip::from_cues`]).
pub fn from_cues(cues: &[(f32, Shape)], length: f32) -> Lip {
    Lip::from_cues(cues.iter().copied(), length)
}

/// The game's shape for one of Rhubarb Lip Sync's mouth shapes
/// (<https://github.com/DanielSWolf/rhubarb-lip-sync#mouth-shapes>), its
/// six basic shapes `A` to `F` and the extended `G`, `H` and `X`, each
/// matched by the sounds it stands for to the shape numbered for the same
/// sounds in `docs/formats/lip.md`. `None` for any other letter.
pub fn rhubarb_shape(letter: char) -> Option<Shape> {
    Some(match letter {
        // Closed: M, B, P.
        'A' => Shape::M,
        // Slightly open, teeth together: most consonants.
        'B' => Shape::S,
        // Open: EH, AE.
        'C' => Shape::Eh,
        // Wide open: AA.
        'D' => Shape::Ah,
        // Slightly rounded: AO, ER.
        'E' => Shape::Oh,
        // Puckered: UW, OW, W.
        'F' => Shape::Oo,
        // Upper teeth on the lower lip: F, V.
        'G' => Shape::F,
        // Tongue raised: L.
        'H' => Shape::L,
        // At rest.
        'X' => Shape::Rest,
        _ => return None,
    })
}
