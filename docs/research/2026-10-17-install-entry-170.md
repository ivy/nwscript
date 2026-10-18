# `rakata install` on entry 170, against the build and against TSLPatcher

2026-10-17. The first real mod through the executor: entry `170-blaster-adjustment` of modshop's KotOR 1 build (JC's Blaster Adjustment v1.0.0, the mod repair 09 is about), option 1 ("Blaster Pistol & Blaster Rifle + Critical", `pistol_rifle.ini`), installed onto the parent checkpoint's tree, then compared with the build's checkpoint for the entry and with TSLPatcher's own install of the same option onto the same tree. [`tslpatcher-oracle.md`](tslpatcher-oracle.md) is the TSLPatcher half of the method.

## Method

The archive under modshop's `games/kotor/data/archives/170-blaster-adjustment/` was unpacked with 7-Zip into a scratch directory. The parent of entry 170 in the chain is checkpoint `255d96be9c32590e` (21,180 files, 17 GB); its tree was copied twice with `cp -a --reflink=always` (0.5 s each) onto an XFS scratch directory, never written in place. The build's checkpoint for the entry is `a0d0f8c2e9adc871`, whose `changed.txt` names one path, `Override/baseitems.2da`.

- `rakata install TSLPATCHDATA --game COPY --option 1 --report REPORT.json`, release build, onto the first copy.
- `conformance/drivers/tslpatcher.sh TSLPATCHDATA COPY2 1 OUT` (the mod's own `INSTALL.exe`, SHA-256 `1a267454…`) onto the second.
- `diff -rq --no-dereference` of each result against the checkpoint's tree and against each other, then the differing 2DA compared cell by cell (columns, row labels, cells) with a reader of the documented `2DA V2.b` layout.

## Results

| | `rakata install` | TSLPatcher |
| --- | --- | --- |
| Wall time | 0.011 s for the process | 16.0 s through the driver (TSLPatcher's own work under 1 s) |
| Operations | 12 `ChangeRow`s on `baseitems.2da`, all written and read back; the two orphan rows reported `RW0301` and skipped; one `RW0303` (a `dietoroll` written twice in `[blaster_carbine]`, the first read) | "Done. All changes have been applied", no warning |
| `Override/baseitems.2da` | SHA-256 `f536b0370b7ca955…`, read back after the write | SHA-256 `1801376a708a929e…` |
| Paths differing from checkpoint `a0d0f8c2e9adc871` | 0: the tree is the checkpoint's, the table byte for byte | 1: `Override/baseitems.2da` |
| Against each other | 1 path, `Override/baseitems.2da`, whose tables are equal: the same columns, labels and cells | |

So the executor's tree equals the build's for this entry, and the one difference from TSLPatcher's tree is the class [`tslpatcher-oracle.md`](tslpatcher-oracle.md) explains: TSLPatcher 1.2.9b appends the changed cells' strings to the string block, leaving its size field stale, where `rakata-2da` writes BioWare's layout, each distinct string once, which is also what the build's patcher wrote. The parity comparison for a 2DA is semantic, and on that comparison the three trees agree.

The report names each operation's target and the hash found there; the twelve records of one table share the hash of the one write:

```text
[2DAList]
  ok      f536b0370b7ca955 override/baseitems.2da   ChangeRow0 on baseitems.2da
  ...
12 operations: 12 written and read back, 0 skipped, 0 failed
```

The target is spelled `override/` because the checkpoint's tree carries modshop's `override` symlink beside `Override/`, and the executor takes the spelling the destination names when the folder exists in that case.
