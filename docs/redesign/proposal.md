# Control panel redesign: audit and proposal

Scope: the Foundation Sunshine control panel (Tauri shell, Vue renderer) and how it relates to the Web UI.
Base: panel branch `feat/ds5-native-host` at `deac5fa`. Nothing here is implemented except the advanced-options
warning (see "Already done"). The three directions are throwaway prototypes in `docs/redesign/prototypes/`,
rendered from real HTML and CSS; screenshots are in `/var/tmp/ds5-panel-redesign-evidence/<variant>/`.

## 1. What is in the panel today

The shell is a sidebar plus one content slot. The first five sidebar entries are Web UI pages shown inside the panel;
everything else is native.

| Sidebar group | Entry | What it is for | Built as |
|---|---|---|---|
| 串流 | 概览 | Host home, counts, links | Web UI page in the slot |
| | 下载客户端 | Client download links | Web UI page |
| | 客户端配对 | PIN pairing, paired clients | Web UI page |
| | 串流应用 | App list and per-app settings (incl. per-app gamepad type) | Web UI page |
| | 串流设置 | Encoder, network, audio and the rest of host config | Web UI page |
| 设备与显示 | 虚拟显示器 | Virtual display driver: modes, HDR calibration, driver and diagnostics | `VddSettings` (own header, own pill tabs, 2 131 lines of Less) |
| | 设备中心 | Six tabs, below | `ControllersHub` |
| 工具与助手 | 工具箱 | Four tool cards: stream timer, latency test (external site), gamepad test, stylus probe | `ControlPanelPage` |
| | AI 助手 | Assistant | `AiAssistant` (own header) |
| footer | 面板设置 | Theme, language, update channel and version, clipboard sync, account, maintenance | `ControlPanelPage` |
| | 帮助与关于 | Links, troubleshooting, version | `ControlPanelPage` |
| (no entry) | 画质增强 | RTX HDR and DLSS NR | `HdrEnhancedManager` (own h1 and tabs); has no sidebar entry |

Device Hub tabs:

| Tab | Contents |
|---|---|
| 概览 | Three cards (controller, microphone, USB) that each link to the tab with the same name |
| 控制器 | Gamepad type picker, per-game overrides, native DualSense status and tuning (presets, common cards, 92 advanced settings), DS4 / motion / DSU group |
| 麦克风 | Redirect status, backend choice (auto, VB-CABLE, USB virtual microphone, off), current device |
| USB 透传 | Client USB forwarding switch and port, shared USB/IP driver card, manual export, mounted devices |
| 组件管理 | USB/IP transport maintenance, input drivers |
| 检测工具 | Controller meta tool, stylus probe |

## 2. Inconsistencies, concrete

Measured from the source at the base commit.

- **Page header: at least five patterns.** Device Hub: a bordered box with icon, the English words DEVICE HUB in a pixel
  font (24 px, weight 400, hard-coded in the template; the Chinese title `设备中心` exists in the i18n table and is not
  used there) and a sentence. Virtual display: icon chip, h2, actions on the right. Quality: h1 at 24 px / 600 with its own
  tab row. Toolbox, settings, help: plain h1 at 23 px and a paragraph. DualSense tuning: a 17 px card title. AI assistant
  and the RTX HDR / DLSS NR component pages: h2 with their own header classes.
- **Tabs: three separate implementations plus segmented controls.** Hub `ChubTabs` (2 px square-cornered boxes, pixel font, rule underneath), virtual display
  `vdd-section-tab` (9 px-radius segmented pills in a separate file), quality `quality-tabs`, and the segmented
  controls and radio groups used inside the DualSense page.
- **Cards and radii.** Hub cards and labelled boxes (square, 2 px border, a `◈` glyph and a rule as the heading), settings
  cards (14 px radius), DualSense cards (0.875 rem), VDD cards (12 px), tiles in the toolbox. The Less and Vue sources
  declare about 20 different `border-radius` values (12 px is used 24 times, then 8 px, 999 px, 14 px, 10 px, 3 px, 9 px,
  11 px, and a dozen others).
- **Typography.** About ten distinct `font-family` declarations: a pixel family (FusionPixel / PixelMplus12 / YouYuan) on the
  hub and parts of other pages, Segoe UI with Microsoft YaHei, a system stack, Microsoft YaHei UI with Noto Sans SC,
  and several monospace stacks. Pixel Chinese at 12 to 14 px is the least legible text in the product.
- **Colour.** Separate token families: `--chub-*` (5), `--vdd-*` (7), `--ds5t-*` (24), `--panel-*` (5), plus Element
  Plus's own, plus `!important` dark overrides in `dialog.less`. The hub is saturated blue with square outlines in light
  mode and brown with rose outlines in dark mode; the settings page is a soft card on a flat ground. The Web UI has one
  set (`--ui-*`) with a light and a dark value each.
- **Controls.** 95 `el-button`, 17 `el-tag`, 10 `el-switch` next to a hand-made button, switch and segmented control in
  the DualSense files, and bracketed text links in the hub (`[ 刷新 ]`). In dark mode the USB tab's switch is a flat pink block
  that looks different from every other switch.
- **Icons.** Element Plus icons in the sidebar, `◈` text glyphs in hub headings, a custom gamepad SVG, Font Awesome in the
  Web UI.
- **Structure repeated or misplaced.**
  - The hub overview repeats the tab bar: three cards that each say "go to the tab above".
  - The shared USB/IP driver card appears on the USB tab and again on the Components tab.
  - The stylus probe is in the toolbox and in the hub's tools tab; the gamepad test is a toolbox card and a hub concern.
  - A controller (something people use every session) sits second of six tabs under an English pixel title, next to
    maintenance pages (components, tools) that people open once.
  - The DualSense tuning page exists twice: the Web UI `views/DualSense.vue` and the panel `DualSenseSettings.vue`
    plus five `dualsense/Tuning*` components. A change such as the advanced-options warning had to be written twice.
- **Layout.** Content widths and paddings differ per page (the hub page is inset, settings are flush left with a 32 px
  gutter, VDD spans the window). The panel's real minimum window is 1024 × 600.

## 3. Why the hub is the wrong category

People come to the panel to do five different things: check that the host is healthy; pair and manage clients and apps;
set up what the client brings (a controller, a microphone, USB devices); set up display and picture; fix something
(drivers, logs, test tools). The hub merges the third and the fifth, and hides the most used item in it.

Two categories are mixed by implementation, not by purpose: "USB/IP transport" is one driver that serves three
features (DualSense, the virtual microphone, USB passthrough), so it was grouped as "USB", and the microphone was put
in a hub of "things with a driver". The user's question is never "which driver?"; it is "my controller does not work"
or "how do I use my client's microphone".

## 4. Proposed information architecture

Principle: navigation is by task, one destination per thing a person sets up, no destination that only lists other
destinations. Status lives next to the thing (a dot in navigation, a chip in the page header), not on a summary page.

| Group | Destination | Holds | Where it is today |
|---|---|---|---|
| 串流 | 概览 | Host state, counts, one status tile per device area, update and maintenance shortcuts | Web UI home, plus the hub overview (merged) |
| | 客户端 | Pairing, paired clients, client download (one place for "get a client and connect it") | 客户端配对 + 下载客户端 |
| | 串流应用 | Apps | unchanged |
| | 串流设置 | Host and stream configuration | unchanged |
| 输入与外设 | 手柄 | Gamepad type, per-game override, native DualSense status, tuning, advanced (warned), DS4 / motion / DSU | hub tab 2 |
| | 麦克风 | Redirect status, backend, test | hub tab 3 |
| | USB 设备 | Forwarding, mounted devices, manual export | hub tab 4 |
| 显示 | 虚拟显示器 | Modes, HDR calibration | unchanged |
| | 画质增强 | RTX HDR, DLSS NR | no entry today |
| 系统 | 组件 | Every driver and transport with install state: virtual gamepad bus, USB/IP transport, virtual microphone, display driver. Each feature page links here when a component is missing. | hub tab 5 (and the USB tab's driver card) |
| | 诊断工具 | Gamepad test, controller meta tool, stylus probe, stream timer, latency test, logs and troubleshooting | toolbox + hub tab 6 + help |
| | AI 助手 | Assistant | unchanged |
| footer | 面板设置 | Theme, language, update, account, maintenance | unchanged |
| | 帮助与关于 | Links, version | unchanged (troubleshooting moves to 诊断工具) |

Result: twelve destinations in four groups (the panel has eleven entries plus a hidden one today); the Device Hub and
the toolbox disappear as names; nothing is more than one click from navigation; the controller is first in its group.
Direction B shows the same map with five top-level entries and a second-level list inside the page, for people who
prefer a shorter sidebar.

Navigation status dots (ready / waiting / needs attention) replace the hub overview. A host that needs attention shows
it on the dot and on the 概览 tile, with the link to the page that fixes it.

## 5. Panel versus Web UI

Today's rule, "stream settings stay in the Web UI", is right but is applied unevenly. Proposed rule:

- **Host configuration, readable and changeable from any browser, belongs to the Web UI**: stream and encoder
  settings, pairing, apps, controller type, DualSense tuning, DS4 / DSU options, microphone backend, USB forwarding
  switch. These are stored on the host and need nothing from the machine the panel runs on.
- **Things that need the local machine belong to the panel**: installing and repairing drivers (elevation, signature
  probe), mounting and releasing USB devices locally, the microphone test, local test tools (gamepad test, stylus
  probe), update of the panel itself, theme and language of the panel, the tray and window.

By that rule the panel's DualSense tuning and controller-type pages are duplicates. The recommended end state is:
the panel's 手柄 page shows the local status (driver signature, USB/IP, pad present) and hosts the Web UI's own
`/dualsense` page for the settings, exactly as it already hosts the stream pages. That removes about 1.5 k lines and the
double implementation, and makes the warning, the translations (20 locales in the Web UI, two in the panel) and the
tuning behaviour a single piece of code. It needs the Web UI page to use the shared tokens (section 6) and a small
frame header so the seam does not show. Until then the panel page keeps its own copy, restyled with the shared
components.

Where the split is not clean today: virtual display settings live in the panel although most of them are host
configuration; they are also the largest panel page. I would leave them in the panel for now (the driver, HDR
calibration and GPU binding are local) and revisit once the shared components exist.

## 6. One design system for both

One token file, taken from the Web UI's `var.css` (names and values kept), plus scales the Web UI lacks. The
prototypes use exactly this in `tokens.css`; light is the blue theme, dark is the warm dark theme.

- **Colour:** `--ui-page-bg`, `--ui-nav-bg`, `--ui-surface`, `--ui-surface-strong`, `--ui-border`, `--ui-text-*`,
  `--ui-accent` (+ text, soft, contrast), status triples for success, warning, danger. Element Plus variables in the
  panel are mapped from these in one file, replacing `--chub-*`, `--vdd-*`, `--ds5t-*` and the dark `!important`
  overrides.
- **Type:** one sans stack (the Web UI's), one mono stack for versions and ids. Page title 22 px / 600, card title
  15 px / 600, body 14 px, caption 12 px. The pixel font is dropped from content; it can stay for the mascot art.
- **Space and shape:** 4 / 8 / 12 / 16 / 24 / 32 / 48; control height 32 px, control radius 8 px, card radius 12 px,
  dialog radius 16 px. No other radii.
- **Components (each exists once, used by every page):** page header (title, one sentence, actions, optional status
  chip); card (icon chip, title, hint, header actions); setting row (label and description left, control right);
  tabs (underline, for sections of one page) and segmented control (for choosing a value); status chip (ready,
  waiting, attention, error) and navigation dot; notice (info, warning); button (primary, default, ghost, danger);
  switch, slider, radio option; dialog with a warning variant (the advanced-options dialog is the first user);
  skeleton and empty state.
- **Icons:** one outline set for both products (the prototypes use the Web UI's Font Awesome set).
- **Motion and focus:** 120 ms for hover and focus only; a visible 2 px focus ring on every control; reduced-motion
  respected.
- **Where it lives:** a small `ui-tokens` package or a single shared CSS file imported by the Web UI build and the panel
  build, so the two cannot drift.

## 7. Three directions

All three use the same architecture (section 4), the same tokens and the same state. Pages shown: overview, gamepad,
microphone, in Chinese, light and dark, at 1440 and 760 px wide (the panel's real minimum is 1024 × 600 and was
also checked), plus full-length captures of the gamepad page.

| | A · Cards | B · Settings list | C · Rail and status bar |
|---|---|---|---|
| Navigation | Grouped sidebar, twelve entries, status dots, collapses to an icon rail when narrow | Five-category sidebar; the destinations of a category are a list inside the page (a segmented bar when narrow) | 76 px rail with labels under the icons, four groups separated by rules |
| Page header | Title and sentence, actions at right | Breadcrumb, large title, sentence | Slim bar: group / title at left, three live status pills at right |
| Content | Two-column cards with icon chips; the layout the owner chose for the settings page and the Web UI tuning page | Flat grouped rows in bordered groups, one column, 720 px | Sections with a label column on the left and controls on the right, hairlines instead of cards, anchor tabs on long pages |
| Feel | Closest to what exists in the Web UI; friendly, a little more chrome | Quietest, closest to a system settings app; least colour | Densest and most status-first; most different from the Web UI |
| Strength | Lowest risk, reuses the chosen design | Smallest sidebar, easiest narrow layout | Always-visible host status; roomy for long pages |
| Weakness | More boxes on a page than B or C | Two navigation levels; the controller is one click deeper than in A and C | Needs a global status store; the rail has fourteen items and scrolls below about 800 px of window height |

Screenshots (each directory holds 21 files: 18 page images, three pages in light and dark at 1440, 760 and 1024 × 640;
two full-length gamepad captures; `contact-sheet.png`, which shows the 1440 and 760 px images):

- `/var/tmp/ds5-panel-redesign-evidence/a-cards/`
- `/var/tmp/ds5-panel-redesign-evidence/b-list/`
- `/var/tmp/ds5-panel-redesign-evidence/c-rail/`

File names are `<page>-<light|dark>-<normal|narrow|min>.png`.

## 8. Recommendation

Direction A. The structural fix (section 4) carries most of the value and does not depend on the look; A is the
look that is already the owner's choice for settings, already built in the Web UI, and shares its component set one for
one, so the panel and the Web UI become one product with the least new design. From C, take only the status pills
in the title bar as a later addition (it is cheap once the status store exists). From B, take the flat list rows for the
pure information pages (components, diagnostics), where cards add nothing.

## 9. Effort and order

Estimates are developer-days for someone who knows the panel; none was measured, and Web UI alignment is listed
separately.

Common foundation, all directions: token file and Element Plus mapping, page shell and navigation, the twelve base
components, light/dark and narrow checks: 6 to 8 days.

Order, which is the same for A, B and C:

1. Tokens, shell, navigation and the new destination list; the Device Hub route is split into its pages with their
   current content (the pages work before they look right). Removes the "DEVICE HUB" header. 4 days.
2. 手柄 page: move controller type, DualSense status and tuning, DS4 / DSU group onto the shared components. 4 days.
3. 麦克风, USB 设备, 组件, 诊断工具 (merging the toolbox and the tools tab). 6 days.
4. 虚拟显示器 (largest: 3.7 k lines of Vue and Less). 6 days.
5. 画质增强 with a sidebar entry, AI 助手, 面板设置, 帮助与关于, welcome. 4 days.
6. Delete the hub styles, the pixel font, the three private token families, the `!important` dark overrides; full
   dark and narrow pass. 3 days.
7. Web UI alignment: shared token file, replace the panel's DualSense copy by the hosted page. 5 to 8 days (can follow).

| Direction | Total (steps 1 to 6) | Extra over the common order |
|---|---|---|
| A · Cards | 27 to 30 days | none |
| B · Settings list | 31 to 35 days | in-page list navigation and its routing, narrow segmented fallback: +4 |
| C · Rail and status bar | 33 to 38 days | global status store, anchor tabs with scroll tracking, label-column section layout for every row type: +6 to 8 |

Each step can ship alone behind the new navigation; the panel is usable after step 2 with the old VDD page still in
place.

## 10. Risks

- Element Plus restyling goes through `element-theme.less` and `dialog.less`, which rely on `!important`; any leftover
  override will show as a wrong colour in one theme.
- `VddSettings` is the biggest unknown; its own tab and card system must be removed, not wrapped.
- Hosting the Web UI page inside the panel gives a visible seam if the Web UI's background image and the panel's flat
  ground differ; a flat variant of the Web UI page (no background art) is needed inside the panel.
- Renaming and merging destinations (客户端, 诊断工具) changes names that documentation and help text use.
- The panel has two languages and the Web UI twenty; shared strings need one source or they will diverge.
- Contrast of accent text on its soft background was taken from the Web UI tokens and was not measured here.
- C's rail at the minimum window height (600) needs scrolling; the others do not, at the cost of width.
- Moving the controller out of the hub changes where existing users look; the old tab names should redirect for a
  release.

## 11. Already done

The advanced-options warning (confirm disabled for five seconds, "do not show again" stored per browser) exists in
the Web UI `views/DualSense.vue` and in both advanced entries of the panel (the DualSense tuning section and the DS4 /
motion / DSU group). The panel entry is also a use of the shared dialog pattern proposed above.

## 12. What was not verified

- The prototypes were rendered in a desktop Chromium, not in the panel's WebView2 runtime on Windows.
- The state shown is realistic but typed in; the prototypes are not connected to the host.
- Colour contrast was not measured; text sizes were checked by eye at 1440, 1024 and 760 px.
- The panel warning was verified in a browser with mocked host commands, not in the installed Tauri build.
- Translations of the new warning text for the 18 Web UI locales other than English and Chinese have not been read
  by native speakers.
- The effort figures are estimates.
