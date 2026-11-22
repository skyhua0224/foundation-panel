# Release executable size

The Windows release target is at most 7,000,000 compressed bytes. Measure the
executable with `xz -9e -T1`, keeping the uncompressed size as well. This is an
LZMA-family packaging proxy, not a measurement of an installer containing the
executable and its other files. Moving assets out of the executable does not
by itself reduce the installed package.

## Baseline evidence

The release build at `d522aa5bb96f950648ad91a14fac074f2b6589f6` produced a
31,464,960-byte executable; `xz -9e -T1` produces 11,781,464 bytes. Its matching
PDB GUID is `935D5281-69D7-4992-92B5-B7F6AB1076BF`, age 1.
[`authority/panel-size-d522-summary.json`](authority/panel-size-d522-summary.json)
records input hashes and the machine-readable summary.

| PE section | Bytes on disk |
| --- | ---: |
| `.text` | 17,313,792 |
| `.rdata` | 12,996,608 |
| `.pdata` | 968,704 |
| `.rsrc` | 110,080 |
| `.reloc` | 58,368 |
| `.data` | 16,384 |

The PDB's section contributions contain 172 complete Brotli streams in
`.rdata`, totalling **5,870,576 bytes** and expanding to 13,331,745 bytes.
The diagnostic checks that each stream consumes its whole contribution.
It does not guess asset sizes from isolated file signatures.

| Embedded content | Compressed bytes |
| --- | ---: |
| Scripts and other text | 2,163,869 |
| PNG | 1,511,812 |
| WebP | 1,375,318 |
| ICO | 442,668 |
| Standalone WOFF2 fonts | 190,933 |
| CSS (including inline font data) | 125,724 |
| JPEG | 48,256 |
| HTML | 8,087 |
| SVG/XML | 3,909 |

Content hashes match the largest streams to the shared UI script
(991,023 bytes), the decorative PNG (671,092), translations (474,959), the
application icon (442,668), the dark hero image (336,603), and shared CSS
(125,724). Identical avatar assets at public and generated asset paths share
the same bytes; their alias paths are not counted as two embedded streams.

Of the remaining 7,126,032 `.rdata` bytes, **4,365,300 bytes** are in 11,604
contributions reached by unwind addresses in the x64 runtime-function table
(80,699 entries; 73,998 unique unwind addresses). These whole contributions
can contain pooled or language-specific records, so this is not the exact
net size of individual `UNWIND_INFO` structures. The remaining 2,760,732 bytes
are **not attributed** between strings, ICU tables, other constants, and
additional metadata. None is guessed to be an ICU data archive.
See `panel-ms-x64-unwind`, RUNTIME_FUNCTION and UNWIND_INFO. Its current APX
Unwind V3 preview does not change this diagnostic's use of table addresses;
the diagnostic does not parse version-specific unwind opcodes.

The baseline PDB has source lines for only **1.705%** of `.text`, mostly the
standard library. Fat LTO put Rust code into one object. Neither its object
name nor its sparse public symbol table supports a reliable top-20 crate
ranking. A build with release line tables is required before publishing that
ranking. The diagnostic reports missing and folded/mixed ranges separately.

## Current baseline and top 20

PR #20 subsequently added release line tables and was merged as
`d09949dc3d2d659a768c9734da0bba195f6c9ad9`. The measured CI source commit
`7ae6b576f01d984ee15e3ae56ecb6f06a0d4815f` has the identical Git tree
`e2f72f6716158b4d5a976db98789ad3899be8992`. CI run `44017029931` succeeded;
its executable is **31,456,256 bytes**, or **11,780,508 bytes** with
`xz -9e -T1`. This is a new baseline, not the result of the feature pruning.
Its PDB GUID is `4A501839-75C8-4683-B567-39720E1C160B`, age 1.
[`authority/panel-size-7ae-summary.json`](authority/panel-size-7ae-summary.json)
records its hashes and full attribution.

Disjoint source-line ranges cover **98.0979%** of its `.text` virtual size
(17,306,448 bytes). The following is attribution to source owners **after
LTO and inlining**, not independent pre-LTO object-file sizes. It leaves
151,416 bytes of other sources, 146,707 folded/mixed bytes, and 31,057 bytes
without line tables separate instead of charging them arbitrarily to a crate.

| Source owner | `.text` bytes |
| --- | ---: |
| sunshine-gui | 4,851,479 |
| tokio 1.52.3 | 2,741,672 |
| tauri 2.12.4 | 2,396,818 |
| core | 1,298,544 |
| std | 656,140 |
| image 0.25.10 | 517,750 |
| alloc | 323,809 |
| regex-automata 0.4.14 | 298,167 |
| serde_core 1.0.228 | 284,118 |
| tauri-runtime-wry 2.12.2 | 211,609 |
| exr 1.74.0 | 193,412 |
| image-webp 0.2.4 | 170,276 |
| zune-jpeg 0.5.15 | 156,133 |
| hyper 1.9.0 | 139,525 |
| tiff 0.11.3 | 139,346 |
| tao 0.37.1 | 133,012 |
| tauri-utils 2.10.2 | 123,382 |
| urlpattern 0.6.0 | 120,812 |
| regex-syntax 0.8.10 | 119,584 |
| aho-corasick 1.1.4 | 117,461 |

The new baseline still contains exactly 5,870,576 bytes of complete Brotli
asset streams. Its unwind-referenced contributions total 4,364,360 bytes;
2,760,648 other `.rdata` bytes remain unattributed.

An additional `llvm-pdbutil dump -symbols -modi=0` inspection found 22,069
Rust-object procedure records. The largest is a `main::inner` closure at
2,334,511 bytes. Neither that procedure inspection nor source attribution
found `ravif` or `rav1e` code. Thus removing the AVIF feature is dependency
pruning; **large executable savings from it are not established**. The main
targets for further measured work are application/framework monomorphization
and already-compressed frontend content, not an assumed large AV1 encoder.

## What round 2 changed

Three changes, each lossless for what the application shows or does.

- **A decorative PNG nothing references.** `src/renderer/public/miside.png`
  (671,087 bytes) was copied into every build. A search of this repository
  and of foundation-ui finds no reference to it (the assistant avatar uses
  `mita-pixel.png`). It is removed. Effect on the application: none.
- **A web icon with 25 frames.** `src/assets/sunshine.ico` carried 25 PNG
  frames from 16 to 256 px (442,663 bytes). It is displayed through `<img>`
  and as the page icon, so the frames in between were never chosen. It keeps
  the seven standard frames 16, 24, 32, 48, 64, 128 and 256 (92,790 bytes).
  The kept frames are the original PNG streams, byte for byte. The tray and
  executable icons are not touched.
- **Image codecs the panel never uses.** The Windows `image` dependency now
  enables `bmp`, `gif`, `ico`, `jpeg`, `png`, `webp` and `rayon`. It drops the
  AVIF encoder and the EXR, TIFF, HDR, DDS, farbfeld, PNM, QOI and TGA
  codecs. The panel reads PNG (clipboard frames, the updater sheet, the DPI
  cover), BMP, JPEG and ICO (tray icons, with format guessing as the
  fallback), and writes JPEG screenshots and PNG/BMP clipboard images; GIF and
  WebP stay so a custom tray image or a pasted web image still decodes. A
  clipboard image or custom tray icon in one of the dropped rare formats no
  longer decodes. See `panel-image-features`, `panel-image-avif`,
  `panel-cargo-features`.

## Measured result

Windows CI builds from the same toolchain, the same foundation-ui commit
(`b7a03fd`) and the same Foundation Sunshine settings data, dispatched from one
fork. "xz" is `xz -9e -T1` of `sunshine-gui.exe`; "installer" is the NSIS
component installer built in the same job.

| Build | Executable | xz | Installer |
| --- | ---: | ---: | ---: |
| `main` before this change | 31,460,352 | 11,793,200 | 14,340,752 |
| this change (`opt-level = "s"`) | 29,193,728 | 10,474,184 | 12,891,820 |
| change | -2,266,624 (-7.2%) | -1,319,016 (-11.2%) | -1,448,932 (-10.1%) |

The same two trees built again after the locked toolchain (rustc 1.100.0) landed on
`main`: executable 31,456,256 -> 29,189,632 bytes, 14,395,710 -> 12,931,756 bytes as
zipped by the CI step, installer 13.67 -> 12.29 MiB. The ratios match the table; the
xz size of these two executables was not taken.

By section: `.text` 17,306,624 -> 16,380,928 (the image codecs: `image` 517,750 ->
189,819 bytes of attributed code, `exr` 193,412 and `tiff` 139,346 gone),
`.rdata` 12,999,680 -> 11,688,960 (the embedded Brotli assets 5,875,900 ->
4,854,931 bytes: PNG 1,511,812 -> 840,720 and the ICO 442,668 -> 92,795),
`.pdata` 968,192 -> 941,056. The compressed target of 7,000,000 bytes is not
met: the executable is 3.47 MB over it, and 4.85 MB of what remains is
already-compressed assets (WebP 1.38 MB, JavaScript 2.17 MB, PNG 0.84 MB)
that no executable-level setting shrinks further.

Rejected, measured the same way (each as its own CI build of the same tree):

| Variant | Executable | xz | Installer | Why not |
| --- | ---: | ---: | ---: | --- |
| `opt-level = "z"` | 24,771,072 | 9,718,896 | 11,760,494 | Hot image paths get slower (below) |
| `panic = "abort"` | 17,220,096 | 8,821,268 | 10,390,512 | A panic in any thread would end the process; see below |
| dependencies `z`, own code `s` | 29,305,856 | 10,489,840 | 12,841,076 | No saving: with fat LTO the merged module is optimised at the level of the final crate |
| thin LTO, dependencies `z`, the image path `s` | 29,606,400 | 10,435,232 | 12,918,179 | 0.4% smaller xz than `s`: no use for the build time |

`opt-level = "z"` does not slow the start: window visible and first content
are within noise of the `s` build. It slows the work done on user actions.
The same `image` calls compiled at `s` and at `z` (4K RGBA frame, same
profile otherwise, median of nine, three alternating rounds): JPEG decode of a
1920x1080 frame 10.9 -> 68.5 ms (6.3x), JPEG encode at quality 60 55.4 ->
88.1 ms (+59%), WebP decode 20.3 -> 34.3 ms (+69%), triangle resize 4K ->
1080p 121.8 -> 157.3 ms (+29%), PNG encode 471.9 -> 551.2 ms (+17%), PNG decode
87.1 -> 84.6 ms (-3%). These run when a screenshot is attached to feedback, a
clipboard image is synced or a DPI cover is drawn. A slowdown above 5% reverts
the change.

`panic = "abort"` would save 1.65 MB of xz because the unwind tables and
cleanup landing pads go. It is not applied: the panic hook would still run,
but a panic would then abort the whole process through the C runtime's fast
fail, which bypasses the unhandled-exception filter the crash reporter also
relies on; every thread that panics today unwinds and the panel keeps running
(the observer lifetime guard in `text_context` and the
`SUNSHINE_TEST_CRASH=panic` check depend on unwinding). Cargo documents the
setting under `panel-cargo-profiles`; Microsoft documents the tables under
`panel-ms-x64-unwind`.

## Startup and first content

Measured on a Windows Server 2025 CI runner with a warm WebView2 profile,
process start to the main window being visible and to its pixels having
changed (more than half of the sampled pixels differ from the first captured
frame), five builds in shuffled order, 36 trials each, the same executable
twice as a noise control. The window is captured with GDI `BitBlt`; no page
instrumentation is used.

| Build | window visible p50 / p95 | content p50 / p95 / mean |
| --- | ---: | ---: |
| before (two copies pooled, n = 72) | 1149 / 1416 ms | 1443 / 1728 / 1506 ms |
| this change (two copies pooled, n = 72) | 1148 / 1442 ms | 1446 / 1733 / 1492 ms |
| `opt-level = "z"` (n = 36) | 1164 / 1340 ms | 1442 / 1610 / 1457 ms |

The two copies of the same executable differ by up to 0.7% at the median, so
differences under about 1% are noise. This change: window -0.1%, content
+0.2% at the median. Both are inside the 2% limit.

## Compatibility checks

- Panic strategy, panic hook, `Location`: the release profile has no `panic`
  key and `crash/` is untouched. A release build started with
  `SUNSHINE_TEST_CRASH=panic` writes a report whose first frame is
  `panic:mod.rs` at offset `0xdd` (line 221, the test panic), the same in the
  `main`, this-change and `z` builds. Null-dereference and `abort` reports are
  produced as before, with a minidump.
- Archived symbols: the PDB written by the same build matches the
  executable's GUID and age (the analysis script checks it), and frames from
  those crash reports resolve through it with `llvm-symbolizer`
  (`test_crash` at `crash/win.rs:376` and `:380`, `run_test_switch`, `install`,
  `main`). The CI job still fails when no PDB is written.
- The windows `image` feature set was checked with `cargo tree`; every
  remaining format is still enabled, and `cargo test --release` for the
  backend ran in the PR build (tray icon decoding, the DPI cover, the
  clipboard paths).
- `sunshine.ico` opens in Pillow and every kept frame decodes.
- WebView2 creation and runtime detection, and everything else in `src-tauri`,
  are unchanged. Windows 10 1803 with its supported WebView2 runtime was not
  run; nothing in this change touches the code that depends on it. Only
  Windows Server 2025 (build 26100) was run.

## Not done, and what is left

The remaining 4.85 MB of embedded assets and 6.8 MB of non-asset `.rdata` do
not move with feature selection. Options and their measured or estimated
effect, for a decision:

- Unwinding is 1.65 MB of xz (measured with `panic = "abort"`), a behaviour
  change as explained above.
- The compression level of the embedded assets is Tauri's fixed quality-9
  Brotli. Storing the assets uncompressed would save about 0.25 MB of xz but
  adds 7.5 MB to the executable.
- Lossless re-encoding of the remaining PNG files saves about 0.1 MB. WebP and
  the hero images are lossy; recompressing them changes pixels.
- Foundation-ui ships five sizes of the mascot and several decorative images.
  Dropping sizes changes what loads on high-DPI screens.
- Tauri's `removeUnusedCommands` needs the CLI build path; CI uses `cargo build`.

## Deliberately unchanged

- Tauri's default Brotli compression stays enabled. `tauri-codegen` 2.7.3 uses
  quality 9 for release (`panel-tauri-codegen-assets`, compression_settings
  and compress_file). Enabling it again cannot save these bytes.
- The `rayon` feature of `image` stays listed. The only code it linked (rayon-core,
  12,719 bytes of attributed code) left the binary with the EXR codec, so it costs nothing.
- Duplicate crate versions in the lock file (base64, getrandom, keyboard-types,
  windows, windows-core) add up to under 25 KB of attributed code; unifying
  them is not worth an incompatible bump.
- Release devtools stay enabled because the application exposes commands
  that open them.
- Reqwest versions are not forced together: a version appearing in the lock
  file does not establish that two versions contribute to the Windows binary.
- The `windows` bindings used by separate dependencies are not forcibly
  replaced with incompatible versions.
- Panic strategy, panic hook, `Location`, release line tables, PDB archiving,
  WebView2 creation and runtime detection remain as before.

Tauri's current size guide (`panel-tauri-size`, Cargo Configuration) recommends
the same stable-toolchain `opt-level = "s"`, LTO and single codegen unit
already selected. Its additional suggestions to abort panics and strip
symbols are deliberately not applied: this application's unwind and archived
PDB requirements take precedence. Its ACL-based `removeUnusedCommands`
option is a further candidate, but the current CI uses `cargo build` directly
and not the CLI path that activates it; no saving from that option is claimed.

## Reproduction and acceptance

Run `node scripts/ci/analyze-panel-size.mjs <exe> <matching-pdb>` with
`llvm-pdbutil` available, or supply its executable as the third argument.
The script checks PDB GUID/age, reports PE sections, attributes disjoint
source-line ranges to crates, and decodes complete Brotli asset streams.
It uses only Node built-ins and LLVM. See `panel-ms-pe-format` (Section Table)
and `panel-llvm-pdbutil` (dump) in the source index.

Acceptance required a before/after Windows release build with the same
toolchain, frontend inputs, optimization profile, and PDB settings. Record
raw and XZ bytes, PDB hash/GUID, and crash-frame symbolization. Measure process
start to visible window and first content on the same Windows installation
in alternating trials, retaining P50/P95/P99. Explain a slowdown above 2%;
revert above 5%. Source inspection or renderer build duration cannot replace
these measurements. The official WebView2 guidance recommends ETW/WPR and
DevTools profiling (`panel-ms-webview2-performance`, WebView2 ETW tracing).
Windows 10 1803 with its supported WebView2 runtime also needs a separate run.

No new Backend, protocol, Adapter, OS call, or instruction-set requirement is
introduced. The implementation stays in Rust/Tauri and uses Cargo's documented
additive feature selection. Not verified: Windows 10 1803 and Windows builds
other than Server 2025 build 26100, real GPUs and displays, and the
installer's upgrade path from an installed panel (the installer is the same
NSIS script with a smaller executable). Other operating systems are unchanged
by the Windows-only feature selection and have not been built or run here.
The size lab (workflows that dispatch the build, time the start and capture
crash reports) is not part of the repository.
