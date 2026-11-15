# NOTICE

Foundation Panel continues the work of [qiin2333/sunshine-control-panel](https://github.com/qiin2333/sunshine-control-panel).
The history of that repository is kept as it was, with its authors and commits, and this repository adds its own commits on top.

The original project is released under the MIT licence, Copyright (c) 2021 Deluze, and that licence stays in [LICENSE](LICENSE)
and applies to the original code and to this one.

Third-party components keep their own licences:

- Tauri, Vue, Vite and the other packages listed in `package.json` and `src-tauri/Cargo.toml`.
- foundation-ui, the Material 3 Expressive component package used by the renderer: MPL-2.0, source at https://github.com/AlkaidLab/foundation-ui (the licence text is `LICENSE` there). Its files stay under MPL-2.0; the rest of this repository stays under the licence above.
- Material Symbols and the Material colour utilities, under their own terms.

The setup UI (`installer-ui`) and the native tool plugins (`native-plugins`: the plugin API and the stylus input diagnostics plugin) are original work of this
repository's contributors and are under the same MIT licence. Checked on 2026-11-15: the git history has one commit for each (`installer-ui` by skyhua0224, the
plugins by Yundi339 in pull request 108 of the original repository), the files carry no copyright or licence header, and none of their lines of code (50 characters
or more, comments left out) occurs in Sunshine, Foundation Sunshine, Moonlight V+ or the other projects of the same authors, apart from three lines of standard
Rust and Tauri boilerplate. Their `Cargo.toml` files said `GPL-3.0-only` without a reason; they say `MIT` now.
