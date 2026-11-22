set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

# dev-env-just: recipes; mise owns tool selection on every OS.
default:
    @just --list

setup:
    mise trust
    mise install
    mise exec -- node scripts/dev-env.mjs setup

doctor:
    mise exec -- node scripts/dev-env.mjs doctor

# Optional host diagnostics when Linux builds use the container.
doctor-native:
    mise exec -- node scripts/dev-env.mjs doctor-native

build:
    mise exec -- node scripts/dev-env.mjs build

test:
    mise exec -- node scripts/dev-env.mjs test

# Windows installer; arguments go to scripts/build-gui-component-installer.ps1 (for example -Beta).
package *args:
    mise exec -- node scripts/dev-env.mjs package {{args}}
