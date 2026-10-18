# The index over the modded build: timings and size, 2026-10-17

The proposal's Performance section sets two targets for the index: the full modded build indexed cold in under a minute on the build host, and incrementally, from a checkpoint's changed paths, in seconds. Both are met. These are the measurements, from a release build of `rakata` with schema 1, on the build host (16 CPUs, 30 GB of memory), with the trees in the page cache from earlier runs.

## Method

The trees are modshop's: the vanilla install, the modded build (the `build` link, which names the last checkpoint), and two checkpoints of the build's chain, the `mod:juhani-clones` checkpoint `ba55d9e0db548e33` and its parent `f107f61067d15feb` (`repair:10-no-minimize-under-proton`), each with its `meta.json` naming its parent and its `changed.txt` the 184 paths it touched.

```bash
rakata index build --game VANILLA --tree vanilla --db vanilla.sqlite
rakata index build --game BUILD --tree build --db build.sqlite
rakata index build --game CKPT/f107f61067d15feb/tree --tree ckpt:f107f61067d15feb --db chain.sqlite
rakata index build --game CKPT/ba55d9e0db548e33/tree --tree ckpt:ba55d9e0db548e33 \
  --parent ckpt:f107f61067d15feb --changed CKPT/ba55d9e0db548e33/changed.txt --db chain.sqlite
rakata index build --game CKPT/ba55d9e0db548e33/tree --tree ckpt:ba55d9e0db548e33 --db cold.sqlite
rakata index digest --db chain.sqlite --tree ckpt:ba55d9e0db548e33
rakata index digest --db cold.sqlite --tree ckpt:ba55d9e0db548e33
```

## Results

| Build | Files walked | Read | Members | Hashes parsed | Reused | Wall time | Database |
| --- | --- | --- | --- | --- | --- | --- | --- |
| vanilla, cold | 402 | 402 | 78,103 | 58,277 | 20,228 | 2.7 s | 382 MB |
| modded build, cold | 6,278 | 6,278 | 98,995 | 65,129 | 40,144 | 5.0 s | 495 MB |
| parent checkpoint, cold | 6,177 | 6,177 | 98,995 | 65,029 | 40,143 | 4.8 s | 494 MB |
| juhani-clones checkpoint, from its parent | 6,278 | 104 | 0 | 103 | 1 | 1.2 s | 540 MB (both trees) |

Of the modded build's cold 5.0 s, reading, hashing (the 11 GB of `Override/` among it), parsing and inserting take 3.2 s, coverage and the indexes 1.0 s, and writing the file 0.75 s. Of the incremental 1.2 s, loading the parent's database into memory takes 0.14 s, carrying 6,174 files' rows over 0.25 s, reading the 104 changed files 0.06 s, and writing the file 0.6 s.

The digest of `ckpt:ba55d9e0db548e33` in the chain database (built from its parent) and in the cold database is the same, `2428ff6d1e3fa56f249cc6ce444a61c0917a5f4310511608fafd4e314a117bc7`, and so is that of the `build` tree, the same folder under another label and root. Computing it takes 2.6 s.

The modded build's rows: `gff_fields` 880,191, `refs` 361,280, `calls` 348,536, `cells` 152,768, `coverage` 120,474, `dialog_links` 112,742, `locations` 105,273, `search` 84,375, `dialog_nodes` 81,089, `resources` 74,327, `contents` 65,129, `strings` 49,019.

Its coverage: 21,479 files and 98,995 archive members have a row; 47,754 are `parsed`, 508 `container`, 57,011 `no_reader` (lip 18,432, tpc 13,562, tga 6,875, txi 6,066, mdx 3,757, mdl 3,745, wav 2,303, wok 1,310, ssf 323, pwk 206, dwk 159, lyt 138, vis 132, ltr 3), 15,201 `outside` and none `failed`.
