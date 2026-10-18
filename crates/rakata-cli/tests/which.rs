//! `rakata which` over the user's own vanilla KotOR 1 install: the texture
//! layer, and the quality pack `--texture-pack` selects. Reads
//! `$RAKATA_GAME_K1` and skips, naming the variable, when it is unset. No
//! game bytes are committed.

use assert_cmd::Command;
use serde_json::Value as Json;
use std::path::{Path, PathBuf};

fn game() -> Option<PathBuf> {
    match std::env::var("RAKATA_GAME_K1") {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: $RAKATA_GAME_K1 is unset; set it to a vanilla KotOR 1 install");
            None
        }
    }
}

fn which(game: &Path, args: &[&str]) -> Json {
    let out = Command::cargo_bin("rakata")
        .unwrap()
        .args([
            "--format",
            "json",
            "which",
            "--game",
            game.to_str().unwrap(),
        ])
        .args(args)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}

fn winner_file(doc: &Json) -> &str {
    let path = doc["result"]["winner"]["location"]["path"]
        .as_str()
        .unwrap();
    path.rsplit('/').next().unwrap()
}

#[test]
fn a_texture_font_names_the_gui_pack_and_a_room_texture_the_quality_pack() {
    let Some(game) = game() else { return };
    let font = which(&game, &["fnt_d16x16.tpc"]);
    assert_eq!(winner_file(&font), "swpc_tex_gui.erf", "{font}");
    assert_eq!(font["result"]["winner"]["layer"], "texturepack");
    assert_eq!(font["result"]["texture_pack"], "tpa");
    assert_eq!(
        font["result"]["winner"]["location"]["member"],
        "fnt_d16x16.tpc"
    );

    let room = which(&game, &["lhr_flr01.tpc"]);
    assert_eq!(winner_file(&room), "swpc_tex_tpa.erf", "{room}");
    let room = which(&game, &["lhr_flr01.tpc", "--texture-pack", "tpb"]);
    assert_eq!(winner_file(&room), "swpc_tex_tpb.erf", "{room}");
    assert_eq!(room["result"]["texture_pack"], "tpb");
    assert_eq!(room["diagnostics"].as_array().unwrap().len(), 0, "{room}");
}

#[test]
fn the_text_report_names_the_pack() {
    let Some(game) = game() else { return };
    let out = Command::cargo_bin("rakata")
        .unwrap()
        .args(["which", "fnt_d16x16.tpc", "--game", game.to_str().unwrap()])
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        text.contains("wins  TexturePacks/swpc_tex_gui.erf:fnt_d16x16.tpc"),
        "{text}"
    );
}
