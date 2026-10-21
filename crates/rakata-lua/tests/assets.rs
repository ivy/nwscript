//! The asset tables: textures, layouts, walkmeshes and models, read from
//! bytes a script builds and from the user's own KotOR 1 install, and
//! written back as the bytes they were read from.
//!
//! The install tests read `$RAKATA_GAME_K1` (a vanilla KotOR 1 install) and
//! skip, naming the variable, when it is unset. Writes go to temporary
//! directories; no game bytes are committed.

use std::fs;
use std::path::{Path, PathBuf};

use rakata::game::{Install, ModuleName, Profile};
use rakata::res::ResId;
use rakata_lua::{Failure, Options, Outcome, Print, run};

fn game() -> Option<PathBuf> {
    match std::env::var("RAKATA_GAME_K1") {
        Ok(root) => Some(PathBuf::from(root)),
        Err(_) => {
            eprintln!("skipped: $RAKATA_GAME_K1 is unset; set it to a vanilla KotOR 1 install");
            None
        }
    }
}

fn run_in(
    dir: &Path,
    script: &str,
    args: &[String],
    game: Option<&Path>,
) -> Result<Outcome, Failure> {
    let path = dir.join("main.lua");
    fs::write(&path, script).unwrap();
    run(&Options {
        script: path,
        args: args.to_vec(),
        game: game.map(Path::to_path_buf),
        profile: Profile::K1,
        print: Print::Capture,
    })
}

fn ok(script: &str, args: &[String], game: Option<&Path>) -> String {
    let dir = tempfile::tempdir().unwrap();
    match run_in(dir.path(), script, args, game) {
        Ok(o) => o.output,
        Err(f) => panic!("{}: {}\n{:?}\n{}", f.code, f.message, f.traceback, f.output),
    }
}

#[test]
fn a_layout_and_its_visibility_read_from_text_write_back() {
    let out = ok(
        r#"
        local text = "beginlayout\r\n   roomcount 1\r\n      m01aa_01a 15.7058 22.2345 0.0\r\n"
          .. "   trackcount 0\r\n   obstaclecount 0\r\n   doorhookcount 1\r\n"
          .. "      m01aa_01a door_16 0 21.1225 20.713 -1.31576 0.707107 0.0 0.0 -0.707107\r\n"
          .. "donelayout\r\n"
        local lyt = rakata.lyt.parse(text)
        local r = lyt:rooms()[1]
        print(#lyt:rooms(), r.model, string.format("%.4f", r.position[1]), lyt:room("M01AA_01A").model)
        local h = lyt:door_hooks()[1]
        print(h.room, h.name, h.unknown, string.format("%.6f %.6f", h.orientation[1], h.orientation[4]))
        print(lyt:to_bytes() == text, lyt:semantic_eq(rakata.lyt.parse(lyt:to_bytes())))
        local vis = rakata.vis.parse("a 1\r\n  b\r\nb 1\r\n  a\r\n")
        print(#vis, vis:rooms()[2].room, vis:visible("A")[1], vis:visible("c"))
        local ok, e = pcall(rakata.lyt.parse, "roomcount 1\n")
        print(ok, e.code)
    "#,
        &[],
        None,
    );
    assert_eq!(
        out,
        "1\tm01aa_01a\t15.7058\tm01aa_01a\n\
         m01aa_01a\tdoor_16\t0\t0.707107 -0.707107\n\
         true\ttrue\n\
         2\tb\tb\tnil\n\
         false\tRE2301\n"
    );
}

#[test]
fn a_txi_is_edited_and_the_binary_formats_refuse_with_their_codes() {
    let out = ok(
        r#"
        local txi = rakata.txi.parse("envmaptexture CM_Baremetal\r\nnumchars 2\r\n")
        print(#txi, txi:get("ENVMAPTEXTURE"), txi:int("numchars"), txi:entries()[1].key)
        print(txi:set("envmaptexture", "CM_dsk"), txi:set("blending", "additive"))
        print(txi:to_bytes() == "envmaptexture CM_dsk\r\nnumchars 2\r\nblending additive\r\n")
        txi:set("bad key", "x")
        print(select(2, pcall(txi.to_bytes, txi)).code)
        for _, f in ipairs({rakata.tpc.parse, rakata.tga.parse, rakata.walkmesh.parse, rakata.mdl.parse}) do
          local ok, e = pcall(f, "\1\1\1\1\1\1\1\1\1\1\1\1\1\1\1\1")
          print(ok, e.code)
        end
    "#,
        &[],
        None,
    );
    assert_eq!(
        out,
        "2\tCM_Baremetal\t2\tenvmaptexture\n\
         CM_Baremetal\tnil\n\
         true\n\
         RE2208\n\
         false\tRE2201\nfalse\tRE2204\nfalse\tRE2501\nfalse\tRE2401\n"
    );
}

/// The bytes the Rust API reads for `name`.
fn read(install: &Install, name: &str, module: Option<&str>) -> Vec<u8> {
    let module = module.map(ModuleName::new);
    install
        .read(ResId::parse(name).unwrap(), module.as_ref())
        .unwrap()
        .unwrap()
        .1
}

#[test]
fn end_m01aa_rooms_read_and_write_back_as_the_install_holds_them() {
    let Some(game) = game() else {
        return;
    };
    let out = tempfile::tempdir().unwrap();
    let dir = out.path().display().to_string();
    let o = ok(
        r#"
        local out = rakata.args[1]
        local g = rakata.Install.open()
        local function save(name, b) rakata.fs.write(rakata.fs.join(out, name:lower()), b) end
        local lyt = rakata.lyt.load(g, "m01aa", "end_m01aa")
        save("m01aa.lyt", lyt:to_bytes())
        save("m01aa.vis", rakata.vis.load(g, "m01aa", "end_m01aa"):to_bytes())
        print(#lyt:rooms(), #lyt:door_hooks(), lyt.file_dependency)
        local nodes = 0
        for _, room in ipairs(lyt:rooms()) do
          local m = rakata.mdl.load(g, room.model, "end_m01aa")
          local mdl, mdx = m:to_bytes()
          save(room.model .. ".mdl", mdl)
          save(room.model .. ".mdx", mdx)
          nodes = nodes + #m:nodes()
          local w = rakata.walkmesh.load(g, room.model, "end_m01aa")
          assert(w.kind == "area")
          save(room.model .. ".wok", w:to_bytes())
        end
        print(nodes)

        local m = rakata.mdl.load(g, "m01aa_01a")
        local n = m:node("object3232")
        local mesh = n.mesh
        print(m.name, m.classification, n.kind, mesh.texture, mesh.lightmap, mesh.stride,
          mesh.offsets.position, mesh.offsets.uv2, #mesh.positions == mesh.vertex_count,
          #mesh.vertex_buffer == mesh.vertex_count * mesh.stride, #mesh.indices == 3 * #mesh.faces)
        print(m:nodes()[1].parent, m:nodes()[n.parent + 1].name == m.name)

        local w = rakata.walkmesh.load(g, "m01aa_01a")
        local surfacemat = rakata.twoda.parse(g:read("surfacemat.2da").bytes)
        print(#w:vertex_bytes() == 12 * #w:vertices(), #w:walkable_faces(surfacemat) == #w:adjacency(),
          w:perimeters()[#w:perimeters()] == #w:perimeter_edges(), w:aabb()[1].face)
        local old = w:set_material(0, 7)
        local edited = rakata.walkmesh.parse(w:to_bytes())
        print(edited:faces()[1].material, edited:semantic_eq(rakata.walkmesh.load(g, "m01aa_01a")))
        w:set_material(0, old)
        save("m01aa_01a.wok", w:to_bytes())
        print(select(2, pcall(w.set_material, w, #w:faces(), 0)).code)
        for _, name in ipairs({"plc_footlker.pwk", "dor_lhr010.dwk"}) do
          local o = rakata.walkmesh.load(g, name)
          assert(o.kind == "object")
          save(name, o:to_bytes())
        end

        local c = rakata.mdl.load(g, "c_bantha")
        local walk = c:animation("CWALK")
        local skin
        for _, node in ipairs(c:nodes()) do skin = skin or node.skin end
        local jaw = walk.nodes[1]
        for _, node in ipairs(walk.nodes) do if node.name == "BTJaw" then jaw = node end end
        local o = jaw.controllers[1]
        print(c.classification, #skin.bones, skin.bind[skin.bones[1]].rotation ~= nil,
          o.kind, o.is_compressed, #o.keys[1].value)
        local mdl, mdx = c:to_bytes()
        save("c_bantha.mdl", mdl)
        save("c_bantha.mdx", mdx)

        local tpc = rakata.tpc.load(g, "LHR_flr02")
        local image = tpc:images()[1]
        local joined = {}
        for _, l in ipairs(image) do joined[#joined + 1] = l.data end
        print(tpc.format, tpc.layout, tpc.frames, #image == tpc.mip_count, table.concat(joined) == tpc:pixels())
        save("LHR_flr02.tpc", tpc:to_bytes())
        local txi = tpc:txi()
        txi:set("bumpmaptexture", "LHR_flr02B")
        tpc:set_txi(txi)
        print(rakata.tpc.parse(tpc:to_bytes()):txi():get("bumpmaptexture"))
        save("lbl_menudarr.txi", rakata.txi.load(g, "lbl_menudarr"):to_bytes())

        local tga = rakata.tga.load(g, "M01aa_01a_lm1")
        print(#tga:rgba() == tga.width * tga.height * 4, tga.image_type)
        save("M01aa_01a_lm1.tga", tga:to_bytes())
        print(select(2, pcall(tga.set_rgba, tga, "x")).code)
        print(select(2, pcall(rakata.lyt.load, g, "no_such_area")).code)
    "#,
        &[dir],
        Some(&game),
    );
    let install = Install::open(&game, Profile::K1).unwrap();
    let lyt = rakata::lyt::Lyt::parse(&read(&install, "m01aa.lyt", Some("end_m01aa"))).unwrap();
    let mut nodes = 0;
    for r in &lyt.rooms {
        let mdl = read(&install, &format!("{}.mdl", r.model), Some("end_m01aa"));
        let mdx = read(&install, &format!("{}.mdx", r.model), Some("end_m01aa"));
        nodes += rakata::mdl::parse(&mdl, &mdx).unwrap().nodes.len();
    }
    let lines: Vec<&str> = o.lines().collect();
    assert_eq!(lines[0], "15\t15\tM01aa.max");
    assert_eq!(lines[1], nodes.to_string());
    assert_eq!(
        &lines[2..],
        [
            "M01aa_01a\tother\ttrimesh\tLHR_flr02\tM01aa_01a_lm1\t40\t0\t32\ttrue\ttrue\ttrue",
            "nil\ttrue",
            "true\ttrue\ttrue\tnil",
            "7\tfalse",
            "RE1805",
            "character\t14\ttrue\t20\ttrue\t4",
            "dxt1\tsingle\tnil\ttrue\ttrue",
            "LHR_flr02B",
            "true\ttruecolor",
            "RE1805",
            "RE0404",
        ]
    );

    // Every file the script wrote is the bytes the install holds.
    let mut names: Vec<String> = fs::read_dir(out.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names.len(), 2 + 3 * lyt.rooms.len() + 2 + 2 + 3);
    for name in &names {
        let module = name.starts_with("m01aa").then_some("end_m01aa");
        assert!(
            fs::read(out.path().join(name)).unwrap() == read(&install, name, module),
            "{name} as Lua wrote it is not the install's"
        );
    }
}

#[test]
fn lights_emitters_and_sabers_read_as_typed_tables() {
    let Some(game) = game() else {
        return;
    };
    let o = ok(
        r#"
        local g = rakata.Install.open()
        local muzzle = rakata.mdl.load(g, "v_muzflash_01"):node("muzflash01")
        local e = muzzle.emitter
        local names = {}
        for _, c in ipairs(muzzle.controllers) do names[#names + 1] = c.name end
        print(e.update, e.render, e.blend, e.texture, e.chunk, e.two_sided, e.name_187,
          table.concat(names, ","):find("birth_rate") ~= nil)
        local sun = rakata.mdl.load(g, "gidy_sun")
        local amb, diff = sun:node("gidy_sun_amb").light, sun:node("gidy_sun_diff").light
        print(amb.ambient_only, diff.ambient_only, amb.shadow, diff.shadow)
        local flares = rakata.mdl.load(g, "fx_explode_03"):node("AuroraLight04").light.flares
        print(#flares, flares[1].texture)
        local blade = rakata.mdl.load(g, "w_dblsbr_001"):node("plane269")
        print(#blade.saber.positions, #blade.saber.uv, blade.saber.words[1], blade.saber.words[2],
          blade.mesh.positions[2][3] == blade.saber.positions[2][3])
        "#,
        &[],
        Some(&game),
    );
    assert_eq!(
        o,
        "Explosion\tBillboard_to_Local_Z\tLighten\tfx_muzflash\tnil\ttrue\tNULL\ttrue\n\
         true\tfalse\tfalse\ttrue\n\
         6\tfx_flare01\n\
         176\t176\t98\t97\ttrue\n"
    );
}
