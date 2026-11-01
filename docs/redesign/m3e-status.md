# Material 3 Expressive migration: status per page

Direction B of the earlier proposal (task-based navigation, settings as grouped lists, centred column) on the Material 3
Expressive foundation described in `m3e-research.md`. "Rebuilt" means the page's markup was rewritten on the shared
components. "Skinned" means markup and behaviour are unchanged and the shared colour, shape, type and motion reach it
through the library skins (`legacy.css`) and the compatibility sheets. "Pending" lists what a rebuild still has to do.

## Control panel

| Page | State | Pending |
|---|---|---|
| Shell: rail with four categories, window controls, route change | rebuilt | bottom bar under 600 px is shared code and rendered, but the Tauri window cannot be narrower than 1024 px today |
| Categories: 串流, 输入与外设, 显示, 系统 | rebuilt (status hero and live status on 输入与外设) | |
| 概览, 客户端, 串流应用, 串流设置 (Web UI inside the panel) | skinned (Web UI) | rebuild of the Web UI home, pairing, apps and configuration (below) |
| 手柄: type, per-game, DualSense status, presets, tuning, advanced, DS4 / motion / DSU | rebuilt, both advanced warnings included | lightbar colour well, curve and trigger previews keep their own inner markup |
| 麦克风, USB 设备, 组件, 诊断工具 | rebuilt | |
| 面板设置, 帮助与关于 | rebuilt | |
| 虚拟显示器 (3.7 k lines) | skinned: column, header, tabs as a connected group, cards, chips, buttons, form controls | rebuild of its eight sections on grouped lists; its own 2 131 lines of Less still carry hard-coded colours that the compatibility sheet overrides |
| 画质增强, AI 助手 | skinned: column, header, cards | rebuild; the assistant keeps its own purple accent in places |
| 首次欢迎页 (panel welcome), 更新对话框 | welcome: untouched apart from tokens; update dialog: skinned through the dialog skin | rebuild |
| Web stream settings (development builds only) | untouched | rebuild or remove |
| Advanced warning dialogs (tuning and emulation) | rebuilt on the shared dialog | |

## Web UI

| Page | State | Pending |
|---|---|---|
| Shell: rail (bottom bar when narrow), utilities, centred column, optional wallpaper with colour seed | rebuilt | |
| DualSense tuning | rebuilt, advanced warning included | |
| Home, client pairing, applications, configuration (7 tabs), password, troubleshooting, downloads, welcome and the setup wizard | skinned: centred column, tokens, buttons, fields, tabs, cards, alerts, modals, switches | rebuild on grouped lists: their scoped styles still hard-code the old blue in places; configuration needs grouped rows per tab; the home dashboard needs a hero and status groups |
| Login | the Web UI has no separate login page: sign-in is the browser's basic authentication; the credential page is the welcome page above | |

## Shared package

Tokens (colour from two brand seeds, shape, spring motion, type, space), Roboto Flex, Material Symbols paths, 19 components,
skins for Element Plus and Bootstrap, tests. Pending: declare it as a `file:` dependency of both consumers and regenerate
their lock files; a published package name; a Storybook-style gallery is in `gallery/` only.
