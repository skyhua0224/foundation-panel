# Development environment

The Linux container currently targets x86-64; other Linux architectures require
a separately verified image. Native Windows and macOS use their own toolchains.

The environment has four layers: language toolchains, locked language dependencies,
native development libraries and one command interface. Application source,
release flags and packaging policy remain owned by their existing build definitions.

## Decisions and authority

Sources were read on 2026-11-17 and are recorded in
[the source index](authority/source-index.tsv).

- `dev-env-mise` (tools) and `dev-env-mise-rust` (Rust): mise selects exact
  versions on each desktop OS. Rust 1.100.0 is the current stable release;
  Node 24.22.3 is the maintained LTS line already used by this application.
  Node 26.13.1 was reviewed; migration to a newer major requires its own
  application compatibility evidence. `rust-toolchain.toml` and package.json
  remain readable by native tools (`dev-env-rustup`, The toolchain file).
- `dev-env-corepack` and `dev-env-npm` (npm ci): Corepack is installed explicitly
  at 0.36.0, and invoked as `corepack npm` because npm shims are not enabled by
  default. packageManager selects npm 11.19.1; npm ci consumes the lock without
  rewriting it. The shared UI is an immutable archive URL with lock integrity,
  replacing the moving sibling directory. Existing source aliases resolve inside
  that same locked package. Settings data retains its existing
  revision pin and is fetched into a disposable dependency cache.
- `dev-env-docker` (Pin base image versions), `dev-env-devcontainer` (Dockerfile
  and workspace mount) and `dev-env-ubuntu-snapshot` (snapshot selection): the
  Linux environment uses an Ubuntu 24.04 digest and a signed dated snapshot.
  Every explicitly installed apt package has an exact version. The snapshot
  freezes transitive package resolution too. The bootstrap CA package and its OpenSSL utility have
  exact versions from the signed base distribution; HTTPS verification stays on.
  CI builds this same Dockerfile and runs the same just recipes.
- `dev-env-libspa08` (system-deps), `dev-env-libspa08-build` (C helpers),
  `dev-env-pipewire-native` (SPA) and `dev-env-gtk` (Linux prerequisites):
  PipeWire and SPA development packages are paired at 1.0.5-1ubuntu3.3 to match
  the existing Rust 0.8 bindings. This is an environment compatibility choice,
  rather than a change to the application audio/capture implementation.
  Moving to current 0.10 bindings with newer system headers is deliberately
  deferred to a dependency migration with native capture tests.
- `dev-env-winget` (WinGetPackage/VSComponents) and `dev-env-webview2`
  (Evergreen distribution): declarative MSVC workloads and the maintained
  WebView2 Runtime replace undocumented machine preparation. Runtime updates
  are intentionally independent of the Cargo-locked SDK. MSVC servicing
  versions remain managed by WinGet; this is not a frozen Windows image.
  `dev-env-vs-components` (Desktop development with C++) verifies the workload,
  latest v143 tool component and the recommended SDK 26100 identifiers for the
  selected supported Visual Studio 2022 line. Native Windows validation and any
  migration to a newer major SDK/toolset remain unverified.
- `dev-env-brew` (Bundle and version limitations) and `dev-env-xcode`
  (Command Line Tools): Brewfile declares system tools, while mise owns
  languages. Homebrew explicitly does not provide a reproducible lock file;
  native macOS SDK and formula updates need validation on macOS.
- `dev-env-just` (recipes and Windows shell): the four commands are shared by
  developers and CI. Node performs orchestration because Node is already a
  required toolchain; it adds no runtime dependency to the application.

The chosen approach follows the official Dev Container and Tauri prerequisite
model. Nix and a fully virtualized Windows/macOS image were considered; neither
is added alongside the mise and native package manager interface.
No comparative provisioning benchmark has been performed.

`dev-env-cargo-env` (Cargo environment variables) and `dev-env-cargo-build`
(`--locked`) govern dependency caches and build output. The Windows entry uses
the existing plugin build script and its existing DLL naming/copy policy, with
that script honouring CARGO_TARGET_DIR. Release profile selection stays in the
existing packaging entry.

## Checks and output

`just doctor` is read-only. It reports expected and actual tool versions and
repair commands. On Linux it also checks actual development package versions;
SPA's stable pkg-config ABI string alone cannot identify the header release.
A host with different native libraries uses `just setup` to build the container;
`just doctor` checks that selected build environment. `just doctor-native` checks
the host and reports native mismatches separately. Setup never downgrades host audio libraries.

Temporary dependencies and Cargo output use PANEL_DEPENDENCY_DIR or a per-checkout
cache under the operating system temporary directory. Set TMPDIR to a sufficiently
large disk on shared builders. CARGO_TARGET_DIR and CARGO_BUILD_JOBS are honoured.
The host entry derives its image tag from the environment definitions so one
checkout cannot accidentally reuse another version. The container receives a separate cache mount and no host devices or service sockets.
The checkout mount is writable for npm output. It is not a sandbox for untrusted code.

## Verification boundary

These files describe provisioning on Windows, macOS and Linux. They do not prove
that every application capability works on those systems. Windows uses MSVC/Rust
and WebView2; macOS uses Rust and Xcode/WebKit; Linux uses Rust, WebKitGTK 4.1 and
paired PipeWire/SPA headers. Record each platform's setup, build, tests and actual
runtime separately in review evidence. Android, iOS and HarmonyOS build environments
are outside this desktop application definition.

The Linux environment and renderer can be provisioned and built with these
files. The current application source still contains unguarded Windows-only
code; the Linux native application build fails and the Linux CI job intentionally
keeps that failure visible. Passing the full Linux acceptance requires a separate
application portability change. The stylus plugin is Windows-only; Linux/macOS
test the portable plugin API and report the stylus suite as skipped.

There is no media or transport hot-path change, no protocol Adapter, and no measured
throughput, latency or resource-isolation improvement. The acceptance metric here
is a fresh setup/build/test result with elapsed time. A passing doctor is insufficient
for application build, hardware behaviour, permission or isolation acceptance.
