# Contributing

The checked-in environment is the build contract. Read
[the environment design](docs/development-environment.md) for version updates and limitations.
Run these commands after cloning this repository. Setup installs dependencies;
it does not launch the application or create a release.

## Linux

Install Docker Engine using its official instructions and ensure the current user
can run `docker version`. Then:

```sh
curl -fsSL https://mise.run | sh
~/.local/bin/mise trust && ~/.local/bin/mise install
~/.local/bin/mise exec -- just setup
```

`just build` and `just test` run in the same image as Linux CI. The first setup
builds that image and downloads locked dependencies. Open the repository in a
Dev Container to work interactively; there `just doctor` checks the container.
Outside it, `just doctor` checks the selected container too. `just doctor-native`
reports host library mismatches and the container repair command.
Docker Desktop is also supported where Docker Engine is unavailable.

## Windows

Use PowerShell with WinGet available. No virtual machine is required.

```powershell
winget install --id jdx.mise --exact; winget install --id Casey.Just --exact
mise trust; mise install
mise exec -- just setup
```

Accept the WinGet configuration prompt. Open Developer PowerShell for the installed
MSVC tools and run `mise exec -- just doctor`, then `just build` or `just test`.
WebView2 Runtime is installed through WinGet; the WebView2 SDK bindings are locked
in Cargo.lock, rather than an unrelated global SDK installation.

## macOS

Install Homebrew from its official instructions, then:

```sh
xcode-select --install
brew install mise just
mise trust && mise install && mise exec -- just setup
```

`just setup` applies Brewfile. Run `just doctor`, then `just build` and `just test`.
The environment definition does not establish native application portability:
macOS application build and runtime behaviour require separate verification.

## Updating locks

Change language versions in their native manifests and mise.toml together.
Regenerate package-lock.json with the selected npm, then run setup, doctor,
build and test in a fresh environment. Rust commands always use `--locked`.
Changes to base image digest, signed package snapshot or apt package versions
must be reviewed together and validated with an uncached image build.
Never repair a build by selecting a different adjacent source checkout.
