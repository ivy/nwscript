# Decompiler: verified fraction by module

How much of KotOR 1's bytecode `nws decompile` recovers to source that recompiles to the original bytes, per module. A function counts as verified when the project's compiler, given the recovered source, emits exactly the function's original instructions; nothing else counts. Unverified functions are recovered but recompile differently, and every one in this corpus is a call against an engine declaration older than the shipped `nwscript.nss` (see [`research/bytecode-quirks.md`](research/bytecode-quirks.md), "Module bytecode compiled against other declarations"). Not recovered would be a function the decompiler could not read; there are none.

`ncs` is the game's `scripts.bif`, including the 30 scripts that ship without source; `modules/NAME_s` is each module's script archive; `rims/` are the global script archives. The vanilla pairs alone, scripts with both source and bytecode, verify at 4,023 of 4,029 functions (99.85%), measured by `crates/nwscript/tests/decompile_corpus.rs`.

Regenerate from a corpus extracted from your own game (`AGENTS.md`):

```bash
cargo build --release -p nws
C=${NWSCRIPT_CORPUS:-~/.cache/nwscript/corpus/k1}
target/release/nws --game k1 --declarations bioware -I "$C" decompile "$C" --markdown table.md
```

`--declarations bioware` reads and verifies against the declarations BioWare's scripts were compiled against; with the shipped declarations, 98.42% of module functions verify.

| Directory | Scripts | Verified whole | Failed | Functions | Verified | Unverified | Not recovered | Unchecked | Verified fraction |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `modules/danm13_s` | 252 | 251 | 0 | 495 | 488 | 7 | 0 | 0 | 98.59% |
| `modules/danm14aa_s` | 107 | 107 | 0 | 215 | 215 | 0 | 0 | 0 | 100.00% |
| `modules/danm14ab_s` | 60 | 60 | 0 | 352 | 352 | 0 | 0 | 0 | 100.00% |
| `modules/danm14ac_s` | 79 | 79 | 0 | 543 | 543 | 0 | 0 | 0 | 100.00% |
| `modules/danm14ad_s` | 59 | 59 | 0 | 179 | 179 | 0 | 0 | 0 | 100.00% |
| `modules/danm14ae_s` | 22 | 22 | 0 | 98 | 98 | 0 | 0 | 0 | 100.00% |
| `modules/danm15_s` | 53 | 53 | 0 | 283 | 283 | 0 | 0 | 0 | 100.00% |
| `modules/danm16_s` | 107 | 107 | 0 | 383 | 383 | 0 | 0 | 0 | 100.00% |
| `modules/ebo_m12aa_s` | 264 | 254 | 0 | 371 | 361 | 10 | 0 | 0 | 97.30% |
| `modules/ebo_m40aa_s` | 19 | 19 | 0 | 40 | 40 | 0 | 0 | 0 | 100.00% |
| `modules/ebo_m40ad_s` | 102 | 102 | 0 | 117 | 117 | 0 | 0 | 0 | 100.00% |
| `modules/ebo_m41aa_s` | 174 | 164 | 0 | 231 | 221 | 10 | 0 | 0 | 95.67% |
| `modules/ebo_m46ab_s` | 24 | 21 | 0 | 42 | 39 | 3 | 0 | 0 | 92.86% |
| `modules/end_m01aa_s` | 188 | 188 | 0 | 972 | 972 | 0 | 0 | 0 | 100.00% |
| `modules/end_m01ab_s` | 83 | 83 | 0 | 301 | 301 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m22aa_s` | 137 | 137 | 0 | 645 | 645 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m22ab_s` | 84 | 84 | 0 | 803 | 803 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m23aa_s` | 20 | 20 | 0 | 88 | 88 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m23ab_s` | 15 | 15 | 0 | 55 | 55 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m23ac_s` | 11 | 11 | 0 | 41 | 41 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m23ad_s` | 44 | 44 | 0 | 396 | 396 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m24aa_s` | 185 | 185 | 0 | 1331 | 1331 | 0 | 0 | 0 | 100.00% |
| `modules/kas_m25aa_s` | 114 | 114 | 0 | 1079 | 1079 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m33aa_s` | 218 | 218 | 0 | 989 | 989 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m33ab_s` | 37 | 37 | 0 | 247 | 247 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m34aa_s` | 56 | 56 | 0 | 687 | 687 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m35aa_s` | 322 | 322 | 0 | 1495 | 1495 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m36aa_s` | 99 | 99 | 0 | 858 | 858 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m37aa_s` | 81 | 81 | 0 | 490 | 490 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m38aa_s` | 41 | 41 | 0 | 317 | 317 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m38ab_s` | 55 | 55 | 0 | 413 | 413 | 0 | 0 | 0 | 100.00% |
| `modules/korr_m39aa_s` | 178 | 178 | 0 | 1186 | 1186 | 0 | 0 | 0 | 100.00% |
| `modules/lev_m40aa_s` | 181 | 181 | 0 | 752 | 752 | 0 | 0 | 0 | 100.00% |
| `modules/lev_m40ab_s` | 123 | 123 | 0 | 570 | 570 | 0 | 0 | 0 | 100.00% |
| `modules/lev_m40ac_s` | 95 | 95 | 0 | 469 | 469 | 0 | 0 | 0 | 100.00% |
| `modules/lev_m40ad_s` | 39 | 39 | 0 | 204 | 204 | 0 | 0 | 0 | 100.00% |
| `modules/liv_m99aa_s` | 111 | 111 | 0 | 206 | 206 | 0 | 0 | 0 | 100.00% |
| `modules/m12ab_s` | 12 | 12 | 0 | 54 | 54 | 0 | 0 | 0 | 100.00% |
| `modules/manm26aa_s` | 102 | 102 | 0 | 162 | 162 | 0 | 0 | 0 | 100.00% |
| `modules/manm26ab_s` | 83 | 83 | 0 | 181 | 181 | 0 | 0 | 0 | 100.00% |
| `modules/manm26ac_s` | 58 | 58 | 0 | 96 | 96 | 0 | 0 | 0 | 100.00% |
| `modules/manm26ad_s` | 85 | 85 | 0 | 439 | 439 | 0 | 0 | 0 | 100.00% |
| `modules/manm26ae_s` | 156 | 156 | 0 | 278 | 278 | 0 | 0 | 0 | 100.00% |
| `modules/manm26mg_s` | 5 | 5 | 0 | 5 | 5 | 0 | 0 | 0 | 100.00% |
| `modules/manm27aa_s` | 140 | 140 | 0 | 425 | 425 | 0 | 0 | 0 | 100.00% |
| `modules/manm28aa_s` | 73 | 73 | 0 | 622 | 622 | 0 | 0 | 0 | 100.00% |
| `modules/manm28ab_s` | 10 | 10 | 0 | 39 | 39 | 0 | 0 | 0 | 100.00% |
| `modules/manm28ac_s` | 19 | 19 | 0 | 118 | 118 | 0 | 0 | 0 | 100.00% |
| `modules/manm28ad_s` | 26 | 26 | 0 | 67 | 67 | 0 | 0 | 0 | 100.00% |
| `modules/sta_m45aa_s` | 127 | 127 | 0 | 1009 | 1009 | 0 | 0 | 0 | 100.00% |
| `modules/sta_m45ab_s` | 42 | 42 | 0 | 240 | 240 | 0 | 0 | 0 | 100.00% |
| `modules/sta_m45ac_s` | 193 | 193 | 0 | 1090 | 1090 | 0 | 0 | 0 | 100.00% |
| `modules/sta_m45ad_s` | 96 | 96 | 0 | 1086 | 1086 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_00_s` | 8 | 8 | 0 | 8 | 8 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_03a_s` | 2 | 2 | 0 | 2 | 2 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_06_s` | 5 | 5 | 0 | 5 | 5 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_07_s` | 1 | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_12_s` | 20 | 20 | 0 | 76 | 76 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_14_s` | 18 | 18 | 0 | 92 | 92 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_16_s` | 1 | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_18_s` | 3 | 3 | 0 | 3 | 3 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_19_s` | 8 | 8 | 0 | 9 | 9 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_31b_s` | 4 | 4 | 0 | 5 | 5 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_34_s` | 1 | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_35_s` | 1 | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_42_s` | 1 | 1 | 0 | 1 | 1 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_44_s` | 5 | 5 | 0 | 5 | 5 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_50a_s` | 2 | 2 | 0 | 3 | 3 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_51a_s` | 4 | 4 | 0 | 5 | 5 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_54a_s` | 2 | 2 | 0 | 3 | 3 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_55a_s` | 6 | 6 | 0 | 7 | 7 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_56a_s` | 6 | 6 | 0 | 7 | 7 | 0 | 0 | 0 | 100.00% |
| `modules/stunt_57_s` | 12 | 12 | 0 | 16 | 16 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m02aa_s` | 165 | 165 | 0 | 621 | 621 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m02ab_s` | 189 | 189 | 0 | 539 | 539 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m02ac_s` | 153 | 153 | 0 | 473 | 473 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m02ad_s` | 124 | 124 | 0 | 575 | 575 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m02ae_s` | 271 | 271 | 0 | 735 | 735 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m02af_s` | 25 | 25 | 0 | 57 | 57 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m03aa_s` | 363 | 363 | 0 | 944 | 944 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m03ab_s` | 327 | 327 | 0 | 440 | 440 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m03ad_s` | 326 | 326 | 0 | 483 | 483 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m03ae_s` | 156 | 156 | 0 | 484 | 484 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m03af_s` | 51 | 51 | 0 | 213 | 213 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m03mg_s` | 7 | 7 | 0 | 10 | 10 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m04aa_s` | 255 | 255 | 0 | 1304 | 1304 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m05aa_s` | 72 | 72 | 0 | 502 | 502 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m05ab_s` | 33 | 33 | 0 | 202 | 202 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m08aa_s` | 110 | 110 | 0 | 566 | 566 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m09aa_s` | 80 | 80 | 0 | 275 | 275 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m09ab_s` | 5 | 5 | 0 | 24 | 24 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m10aa_s` | 369 | 369 | 0 | 688 | 688 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m10ab_s` | 344 | 339 | 0 | 382 | 377 | 5 | 0 | 0 | 98.69% |
| `modules/tar_m10ac_s` | 365 | 365 | 0 | 661 | 661 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m11aa_s` | 64 | 64 | 0 | 241 | 241 | 0 | 0 | 0 | 100.00% |
| `modules/tar_m11ab_s` | 7 | 7 | 0 | 114 | 114 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17aa_s` | 91 | 91 | 0 | 446 | 446 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17ab_s` | 47 | 47 | 0 | 341 | 341 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17ac_s` | 36 | 36 | 0 | 111 | 111 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17ad_s` | 66 | 66 | 0 | 218 | 218 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17ae_s` | 124 | 124 | 0 | 372 | 372 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17af_s` | 51 | 51 | 0 | 159 | 159 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17ag_s` | 37 | 37 | 0 | 87 | 87 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m17mg_s` | 6 | 6 | 0 | 6 | 6 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m18aa_s` | 181 | 181 | 0 | 1843 | 1843 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m18ab_s` | 75 | 75 | 0 | 837 | 837 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m18ac_s` | 114 | 114 | 0 | 778 | 778 | 0 | 0 | 0 | 100.00% |
| `modules/tat_m20aa_s` | 87 | 87 | 0 | 570 | 570 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m41aa_s` | 129 | 128 | 0 | 682 | 681 | 1 | 0 | 0 | 99.85% |
| `modules/unk_m41ab_s` | 59 | 59 | 0 | 387 | 387 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m41ac_s` | 85 | 85 | 0 | 534 | 534 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m41ad_s` | 147 | 147 | 0 | 489 | 489 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m42aa_s` | 112 | 112 | 0 | 832 | 832 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m43aa_s` | 95 | 95 | 0 | 397 | 397 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m44aa_s` | 176 | 176 | 0 | 782 | 782 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m44ab_s` | 33 | 33 | 0 | 110 | 110 | 0 | 0 | 0 | 100.00% |
| `modules/unk_m44ac_s` | 75 | 75 | 0 | 306 | 306 | 0 | 0 | 0 | 100.00% |
| `ncs` | 1784 | 1778 | 0 | 4191 | 4184 | 7 | 0 | 0 | 99.83% |
| `rims/global` | 246 | 246 | 0 | 793 | 793 | 0 | 0 | 0 | 100.00% |
| `rims/miniglobal` | 246 | 246 | 0 | 793 | 793 | 0 | 0 | 0 | 100.00% |
| **total** | 13139 | 13103 | 0 | 50702 | 50659 | 43 | 0 | 0 | 99.92% |
