# Material 3 Expressive migration: status per page

Updated 2026-11-02. "Rebuilt" = the page's markup was rewritten on the shared components and the row model (tinted page,
lighter group cards, no carets, plain outline icons, one short second line, centred column, small header). "Skinned" =
markup and behaviour unchanged, shared colour/shape/type/motion reach it through `legacy.css`. "Pending" says what is left.
Evidence: `/var/tmp/ds5-panel-redesign-evidence/m3e/rules3/` (screens), `refs/` and `refs-found/` (owner references),
`google-settings-study.md` (values and sources).

## Shell (both consumers)

Scene model after Navigation 3 (`src/nav/`): one back stack (`createNav`), scene strategies (single pane, list + detail,
list + detail + supporting), the persistent rail wraps every scene (`MScene` + `MRail`): under 600 bottom bar; 600 to 720
collapsed rail, one pane; 720 to 1200 rail, list, detail; 1200 and over supporting pane for pages that declare one; 1600 and
over expanded rail with labels. List pane 0.3636 of the space, clamped 240 to 440, drag handle with persisted width.
Panes stay mounted: resize never reloads a page, never loses the stack, scroll or an edit. Keyboard: Esc/Alt+Left back, `/` or
Ctrl+F search, Ctrl+1..9 sections, `?` shortcut sheet. Container-width based (ResizeObserver), not window media queries.

## Control panel

| Page | State | Pending |
|---|---|---|
| Shell: rail, list pane, search, detail, supporting pane, bottom bar, window minimum 480 x 560 | rebuilt | the window minimum is in `src-tauri/src/windows.rs` (not compiled on this host); search covers pages, not single settings |
| 概览 (tile page, edit mode, add sheet, reset, persisted, context menus) | rebuilt, real sources only | CPU/GPU load and temperature only exist through HWiNFO (`hwinfo_*`): not in the catalogue; packet loss and codec have no source (below) |
| 手柄 (supporting pane with live state) | rebuilt, both advanced warnings kept | |
| 麦克风 | rebuilt (status capsule, banners) | |
| USB 设备, 组件, 诊断工具 | rebuilt shell and page header; the panels inside (`controllersHub/*`) are on grouped rows | section descriptions moved behind info icons; a few long sentences remain |
| 帮助与关于 (promo cards: downloads, sponsor sheet), 面板设置 | rebuilt | |
| 更新 (sheet, wavy progress, step checks) | rebuilt | the sheet was not run against a real update |
| 画质增强 (overlay, session, shortcuts, components) | rebuilt, `EnhancementManager` on rows | the sketch art of the component cards was removed |
| AI 助手 (chat, configuration sub-page with master switch) | rebuilt | |
| 虚拟显示器 (overview, display, calibration, driver sub-pages) | rebuilt, no hard-coded colours (old Less removed) | checked with the harness mock only, not with a real driver |
| Web 串流服务 (Moonlight Web, from the tray) | rebuilt | checked with the harness mock only |
| 首次欢迎页 | rebuilt | |
| Web UI pages inside the panel (客户端, 串流应用, 串流设置, 下载, 密码, 故障诊断) | the Web UI pages, embedded: the Web shell hides itself in a Tauri frame | the single-pane back arrow of embedded pages is a small bar from `PanelWebHost` |

## Web UI

| Page | State | Pending |
|---|---|---|
| Shell (WebShell: icon-only rail foot, theme and account in the page header under 600 px, live second lines, list action button, row context menu, the old navbar removed) | rebuilt | second lines exist for 客户端, 应用程序 and 配置 (encoder) only: the other pages have no live source |
| Home (tile dashboard with 18 tiles, 13 by default, edit mode, add sheet, reset, persisted per browser; sponsor sheet, resource sheet, version sheet) | rebuilt, real sources only | CPU/GPU load, packet loss and codec have no source; session, latency, bitrate, frame rate and HDR tiles are hidden where the host answers 403 |
| 客户端 (device rows with switch and gear, device settings sheet, PIN pad pairing, QR pairing, context menu) | rebuilt, every setting kept | device type is guessed from the name by a documented keyword table only (`utils/homeData.js`) |
| 下载客户端 (platform cards by device group, one sheet per client with QR, links and mirror) | rebuilt, same links and release resolution | the long per-platform description is no longer shown (features and note are) |
| 故障排除 (action rows with icons and results, promo cards, log table with level counts as toggle filters and match-mode buttons, diagnosis in a side sheet with check rows) | rebuilt | log rows come from the host line format `[time]: Level: message`; a line that does not match joins the row before it; the table shows the newest 2000 lines the page already kept |
| 串流应用 (cover grid and list, drag or menu reorder, batch select, editor / cover finder / scan sheets, command tables, savebar) | rebuilt, every editor field kept | touch drag reorder is replaced by the card menu (move earlier, later); the auto-detach mark no longer shows on cards (it is the default) |
| 配置 (7 tabs: one list-pane row each under 设置; tab page of grouped rows; long sections are pages of their own; floating save bar; per-setting search) | rebuilt, every setting and its conditional visibility kept | the display-preparation cards keep their own preview markup (icons now Material); Webhook settings are rebuilt (one row opens a side sheet with the grouped fields) |
| DualSense (preset button group, one sub-page per everyday card and per advanced group, thick sliders with inset icons and value bubble, centred variant for offsets, range variant for dead zones and trigger travel, log scale, colour swatches, supporting pane with the state and previews from the edited values; both advanced warnings unchanged) | rebuilt | its settings come from a host schema and are not in the search index; the previews (trigger, response curve) keep their own markup |
| 密码 (grouped field rows, illustration, pill button) | rebuilt | |
| 欢迎页 (first-run credentials, standalone: title, illustration, two icon rows, grouped fields, pill button, language pill) | rebuilt | |
| 设置向导 (five steps on the welcome pattern: back bubble, step bar, illustration, selectable rows, pill button; completion toasts, restart countdown) | rebuilt, every step and call kept | the client rows open their links through `openExternalUrl` (the old cards were plain links) |
| 版本卡 (version sheet on Home) | rebuilt | |

## Sponsor

赞助 targets come from commit e11de7c7 and `.github/FUNDING.yml`: `https://www.ifdian.net/a/Yundi339` and
`https://www.ifdian.net/a/qiin2333`, kept in one constant per consumer (`config/sponsors.js`). The tile and the help row open a
sheet with one entry per developer.

## Data left out on purpose

Packet loss and per-session codec: the host receives client-reported loss through the ABR feedback API
(`src/nvhttp/abr_api.cpp`, `src/streaming/abr.cpp`) but no route reads it, and `session_info_t` does not carry the codec.
Searched: `stream.cpp` (IDX_LOSS_STATS is logged only), `src/streaming/perf_recorder.*`, `src/confighttp/runtime.cpp`,
`abr*.cpp`, `git grep videoFormat`, `session_info_t` in `src/stream.h` (ends at `app_id`, no codec). Commands and output: `/var/tmp/ds5-panel-redesign-evidence/m3e/no-source-searches.txt`. Needs an owner decision (a read-only route).

## Shared package

Tokens (HCT colour from the brand seeds with fixed roles, shape, AOSP radius and space scales, spring motion, type with a
desktop step), Roboto Flex, Material Symbols paths, tests. Components: MScene, MRail, MNavList, MSearch, MPage (small
header, `cols`, `wide`), MGroup/MItem/MSection/MFooter, MSwitch, MSlider, MSeg (connected and toggle groups), MSplitButton,
MFabMenu, MToolbar, MProgress (wavy), MLoading, MBanner, MSheet, MDialog, MBadge, MSnackbar, MCoverGrid, MCarousel, MTable,
MActions, MPromo, MPinPad (digits, dots with a morphing active shape, keyboard), MTile/MTileGrid/useTiles, MStatusPill, MRing, MBar, MCheckItem, MMasterSwitch, MSplitItem, MContextMenu,
MChoiceItem, MStat, MSparkline, MGauge, MSwatches, MInfo. Added in pass 3: `MWelcome`, `log` scale on `MSlider`/`MSliderItem`, trail slot on `MChoiceItem`, `info` state on `MCheckItem`, tip card (`.m-tip`), full-width pill for form pages (`.m-page__cta`), save bar pinned to the foot of short pages. Pending: declare it as a `file:` dependency of both consumers and
Both consumers declare the package as a `file:../ds5-ui-m3e` dependency with regenerated lock files (build resolves it through an alias to the sibling checkout, `FOUNDATION_UI_DIR`). Windows WebView2 render: see the report (it depends on the shared VM lock).

## Component per page

### Control panel

| Page | Expressive components |
|---|---|
| Shell | MScene, MRail (collapsed, expanded, bottom bar), MNavList, MSearch, MContextMenu, MSheet (shortcut sheet), overlay slot for the embedded Web frame |
| 概览 | MPage (wide), MTileGrid, MTile, useTiles, MStatusPill, MBar, MSparkline, MSheet (sponsor), MDialog (confirm), MContextMenu, MIconButton |
| 手柄 | MPage (sub-pages), MChoiceItem, MSeg (icons, preset), MItem, MOverflow, MDialog, MFooter, MStatusPill (supporting pane `GamepadState`) |
| 麦克风 | MStatusPill, MBanner, MSeg, MChip, MFooter, MItem |
| USB 设备, 组件, 诊断工具 | MPage, MSection (info tooltip for the description), MGroup, MItem, MSwitchItem |
| 虚拟显示器 | MPage (sub-pages overview, display, calibration, driver), MStatusPill, MProgress, MCheckItem, MTile, MChip, MSeg, MChoiceItem, MSwitchItem, MFieldItem, MBanner, MFooter, savebar |
| 画质增强 | MPage (sub-pages), MSwitchItem, MSlider (size m, inset icons, value bubble), MChoiceItem, MBanner, MStatusPill, MEmpty, MFooter; component cards: MSection, MStatusPill, MBanner, MSwitchItem, MItem |
| AI 助手 | MPage (chat and configuration sub-page), MMasterSwitch, MChoiceItem, MFieldItem, MStatusPill, MBanner, MLoading, MButton |
| Web 串流服务 | MPage (sub-page), MStatusPill, MProgress, MSwitchItem, MSliderItem, MChoiceItem, MFieldItem, MIconButton |
| 帮助与关于 | MPromo (downloads, sponsor), MSheet, MGroup, MItem, MFooter |
| 面板设置 | MSeg (icons), MGroup, MItem, MChip, MButton |
| 更新 | MSheet (persistent), MStatusPill, MProgress (wavy), MCheckItem, MBanner, MButton |
| 首次欢迎页 | MPage, MBanner, MFieldItem, MChoiceItem, MButton |

### Web UI

| Page | Expressive components |
|---|---|
| Shell | MScene, MRail, MNavList, MSearch, MFab (list action), MContextMenu, md-menu (theme, account), MSnackbar, MButton (logout dialog); supporting pane slot |
| Home | MPage, MTileGrid, MTile, useTiles, MChip, MSparkline, MBanner, MSheet, MButton, MIconButton, MSnackbar, MContextMenu |
| 客户端 | MPage, MSection, MSwitch, MSplitItem rows (own markup with a switch), MSheet, MPinPad, MSeg, MField, MFieldItem, MChoiceItem, MGroup, MItem, MFooter, MBanner, MEmpty, MChip, MSnackbar |
| 下载客户端 | MPage, MSection, MSheet, MSeg, MChip, MGroup, MItem, MFooter, MButton (cards are page CSS) |
| 故障排除 (promos) | MPromo, MSheet, MGroup, MItem, MFooter |
| 串流应用 | MPage (wide), MButton, MOverflow, MSeg (view), MCoverGrid (select, reorder, context), MGroup rows with split gear, MEmpty, MSkeleton, MProgress, MSheet (editor, cover finder, scan results, scan options, environment variables), MDialog (delete, discard), MSection, MSwitchItem, MChoiceItem, MFieldItem, MField, MInfo, MTable (command lists and environment variables), MChip, MBanner, MSnackbar, MContextMenu (shell), savebar |
| 配置 | MPage (cols), MSection, MGroup, MItem rows (sub-page rows), MSwitchItem, MChoiceItem (radio dialog with help), MFieldItem, MSliderItem, MField, MFooter, MBanner, MChip, MTable (ports), MInfo (help behind an icon), MSnackbar, MSkeleton, savebar; the Cfg* wrappers (`components/cfg`) map one configuration key to one row |
| 故障排除 | MPage, MSection, MGroup, MItem (action rows with the button at the end), MButton, MNotice, MPromo (HelpPromos), MActions (log actions), MField, MSeg (match mode, ignore case, level filters as toggle group), MTable, MChip (level), MSheet (diagnosis), MCheckItem (findings, fixes), MBanner, MLoading, MIconButton |
| DualSense | MPage (back bubble on sub-pages), MStatusPill, MSection, MSeg (presets), MGroup, MItem, MSwitch, MSlider (size m, inset icons, centred, range, log), MChoiceItem (with trail slot), MChip, MIconButton, MSwatches, MField, MSwitchItem, MBanner, MSnackbar, MFooter, MEmpty, MSkeleton, savebar; supporting pane: MPage, MStatusPill, MItem |
| 密码 | MPage, MSection, MGroup, MFieldItem, MFooter, MBanner, MButton (pill in `m-page__cta`) |
| 欢迎页 | MWelcome (title, illustration, icon rows, actions), MGroup, MFieldItem, MBanner, MButton |
| 设置向导 | MWelcome (back bubble, step bar, illustration, actions), MGroup, MItem (selectable rows), MChoiceItem, MSection, MBanner, MPromo, MButton, MProgress, MIcon, MIconButton, ConfirmDialog (MDialog) |
| Webhook, 版本卡 | MGroup, MItem, MChip, MSheet, MSwitchItem, MFieldItem, MSection, MButton, MBanner, MFooter, MLoading, MSnackbar, tip card |
