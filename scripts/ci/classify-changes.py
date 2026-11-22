#!/usr/bin/env python3
"""Classify changed files into the checks they can affect.

Outputs (key=true/false, one per line, appended to --output):
  rust      compile and test the Rust crates and their build inputs
  renderer  run the renderer tests
  code      build the renderer dist and run the Windows Rust checks (rust or renderer)
  full      the event itself asks for everything (main, tags, manual runs)

A pull request with no list (the API failed or the change is too large to list) is treated as a
full run. Files under docs and Markdown files alone run no compile or test jobs.
"""

import argparse
import sys

DOCUMENTATION_ONLY = (
    "docs/",
    ".github/release-notes/",
)

DOCUMENTATION_NAMES = {
    "LICENSE",
    "NOTICE.md",
    "README.md",
    "CONTRIBUTING.md",
}

# Anything that changes the build contract, the tools, or the workflow itself runs every check.
FULL_INPUTS = (
    ".github/",
    ".devcontainer/",
    "scripts/ci/",
)

FULL_NAMES = {
    "justfile",
    "mise.toml",
    "rust-toolchain.toml",
    "configuration.dsc.yaml",
    "scripts/dev-env.mjs",
    "package.json",
    "package-lock.json",
}

RUST_INPUTS = (
    "src-tauri/",
    "native-plugins/",
    "tests/",
)

RUST_NAMES = {
    "Cargo.lock",
    "Cargo.toml",
    "scripts/build-native-plugins.ps1",
    "scripts/build-gui-component-installer.ps1",
    "scripts/package-gui-bundle.ps1",
    "scripts/deploy-gui.ps1",
}

RENDERER_INPUTS = (
    "src/",
    "assets/",
    "installer-ui/",
)

RENDERER_NAMES = {
    "index.html",
    "vite.config.js",
    "vite.home.config.js",
    "vite.installer.config.js",
    "vite.foundation-ui.test.js",
}


def is_documentation(path):
    return path.endswith(".md") or path in DOCUMENTATION_NAMES or path.startswith(DOCUMENTATION_ONLY)


def classify(paths, event):
    if event != "pull_request" or not paths:
        return {"rust": True, "renderer": True, "code": True, "full": True}

    rust = renderer = False
    for path in paths:
        if is_documentation(path):
            continue
        if path.startswith(FULL_INPUTS) or path in FULL_NAMES:
            rust = renderer = True
        elif path.startswith(RUST_INPUTS) or path in RUST_NAMES:
            rust = True
        elif path.startswith(RENDERER_INPUTS) or path in RENDERER_NAMES:
            renderer = True
        else:
            # An unrecognised build input must not silently skip a check.
            rust = renderer = True

    return {"rust": rust, "renderer": renderer, "code": rust or renderer, "full": False}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--event", default="")
    parser.add_argument("--output", default="-")
    args = parser.parse_args()

    paths = [line.strip() for line in sys.stdin if line.strip()]
    result = classify(paths, args.event)

    lines = [f"{key}={'true' if value else 'false'}" for key, value in result.items()]
    text = "\n".join(lines) + "\n"
    if args.output == "-":
        sys.stdout.write(text)
    else:
        with open(args.output, "a", encoding="utf-8") as handle:
            handle.write(text)

    print(
        f"classified {len(paths)} file(s): rust={result['rust']} "
        f"renderer={result['renderer']} full={result['full']}",
        file=sys.stderr,
    )


if __name__ == "__main__":
    main()

