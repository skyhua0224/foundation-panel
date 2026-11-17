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

## Feature selection

The Windows `image` dependency now explicitly retains every previously
enabled decoder and Rayon, while dropping only the `avif` encoder feature.
PNG/BMP/JPEG clipboard images, PNG updater frames, PNG DPI covers, JPEG
screenshots, ICO tray icons, and fallback decoding of custom tray images keep
their existing format support. The application never requests AVIF output.
`image` 0.25.10 requires **`avif-native`** for AVIF decoding, which was already
disabled. Thus dropping **`avif`** removes an unused encoder, not an existing
decoder. See `panel-image-features`, `panel-image-avif`, and
`panel-cargo-features` (Feature unification) in the source index.

The measured post-change executable size and Windows runtime verification
remain pending. A dependency disappearing from the Windows feature graph is
not evidence of a particular number of bytes disappearing from the linked
executable.

The following remain deliberate:

- Tauri's default Brotli compression stays enabled. `tauri-codegen` 2.7.3 uses
  quality 9 for release (`panel-tauri-codegen-assets`, compression_settings
  and compress_file). Enabling it again cannot save these bytes.
- Rayon stays enabled, preserving existing image-processing scheduling.
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

Acceptance requires a before/after Windows release build with the same
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
additive feature selection. Real-device behavior, resource authorization,
cross-tenant isolation, startup latency, and the compressed-size budget are
not yet verified for this change. Other operating systems are unchanged by
the Windows-only feature selection and have not been built or run here.

Further candidates are lossless image recompression, icon-frame deduplication,
and reducing the large shared UI and translation chunks while preserving
their contents. Their byte savings and runtime effects have not been measured;
they are proposals, not claimed improvements. No competitor performance
comparison or best-in-class claim is made by this packaging change.
