# Material 3 Expressive for the panel and the Web UI: research and stack decision

Everything below was read from the primary sources linked; "not confirmed" marks what could not be read from a
primary source.

## 1. What Material 3 Expressive is, from primary sources

| Topic | What Google publishes | Source |
|---|---|---|
| Scope | Expressive is "an expansion of Material Design 3, including research-backed updates to theming, components, motion, typography, and more"; Compose implements "Material You and Material 3 Expressive"; it "complements the Android 16 visual style and system UI". | [Compose Material 3 guide](https://developer.android.com/develop/ui/compose/designsystems/material3) |
| Research | 46 studies, hundreds of designs, 18 000+ participants; key elements found up to four times faster; preference up to 87 % among 18 to 24 year olds. Caveats given by Google: unfamiliar layouts scored lower (a playlist without its vertical list), removing text labels hurt usability, and a strong minority preferred calmer designs. | [Google Design, Expressive research](https://design.google/library/expressive-material-design-google-research) |
| Colour | Dynamic colour from one source colour in HCT; roles include primary, secondary, tertiary, error, six surface container levels, surface dim and bright, and fixed roles. "Components now use the tonal surface containers by default, not elevation overlays." | [Color.md, Material Components for Android](https://github.com/material-components/material-components-android/blob/master/docs/theming/Color.md) |
| Shape | Corner scale: none 0, extra small 4, small 8, medium 12, large 16, large increased 20, extra large 28, extra large increased 32, extra extra large 48, full 50 %. | [Shape.md](https://github.com/material-components/material-components-android/blob/master/docs/theming/Shape.md) |
| Motion | Two sets of springs exist. Standard: default spatial damping 0.9 / stiffness 700, fast 0.9 / 1400, slow 0.9 / 300. Expressive: default spatial 0.8 / 380, fast 0.6 / 800, slow 0.8 / 200. Effects springs are critically damped (1.0) in both: 1600, 3800, 800. Spatial springs (position, size, shape) overshoot, effects springs (opacity, colour) do not. The older curve tokens (standard cubic-bezier(0.2, 0, 0, 1) and the emphasized ones, durations 50 ms to 1000 ms) remain. | [ExpressiveMotionTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/ExpressiveMotionTokens.kt), [StandardMotionTokens.kt](https://github.com/androidx/androidx/blob/androidx-main/compose/material3/material3/src/commonMain/kotlin/androidx/compose/material3/tokens/StandardMotionTokens.kt), [Motion.md](https://github.com/material-components/material-components-android/blob/master/docs/theming/Motion.md) |
| Type | Emphasized variants step the weight up (display, headline, title large: regular to medium; title medium and small, label: medium to bold; body: regular to medium). Sizes: display 57 / 45 / 36, headline 32 / 28 / 24, title 22 / 16 / 14, body 16 / 14 / 12, label 14 / 12 / 11. The Wear OS guide says Expressive replaces Roboto with Roboto Flex. Line heights were not in the fetched page (the baseline values were used; not confirmed). | [Typography.md](https://github.com/material-components/material-components-android/blob/master/docs/theming/Typography.md), [Wear OS design language](https://developer.android.com/design/ui/wear/guides/get-started/design-language) |
| Components added | Connected button group (replaces the segmented button; inner corners 8 dp, spacing 2 dp, outer fully rounded; buttons of five sizes; 15 % child size change on press), standard button group (12 dp spacing), split button, loading indicator (morphs its shape; under five seconds of waiting; default 38 dp, container 48 dp; replaces most indeterminate circular indicators), floating toolbar, FAB menu, navigation rail, wavy progress. Lists come as standard and segmented, with first / middle / last / single positions that change corner shape. | [ButtonGroup.md](https://github.com/material-components/material-components-android/blob/master/docs/components/ButtonGroup.md), [LoadingIndicator.md](https://github.com/material-components/material-components-android/blob/master/docs/components/LoadingIndicator.md), [List.md](https://github.com/material-components/material-components-android/blob/master/docs/components/List.md), [component index](https://github.com/material-components/material-components-android/tree/master/docs/components) |
| Android 16 | Google's own description: the shade background is "subtly" blurred "to provide a sense of depth", "a system of more natural, springy animations", "emphasized typography", updated dynamic colour themes and responsive components; first on Pixel. | [Google blog, 13 May 2025](https://blog.google/products-and-platforms/platforms/android/material-3-expressive-android-wearos-launch/) |
| Compose status | Button group, split button and floating toolbar APIs are promoted out of experimental in the 1.5 alpha line (alpha20 to alpha22, May and June 2026); the stable release listed is 1.4.0, beta 1.5.0-beta02. Loading indicator, FAB menu and shape morphing were not found in the part of the notes that could be read. | [Compose Material 3 release notes](https://developer.android.com/jetpack/androidx/releases/compose-material3) |

## 2. Android 17

Google's published Android 17 material was read: the developer overview, the "Android 17 is Here" post of 16 June 2026
and the Google I/O developer summary. **None of them describes a new visual design.** They cover adaptive-first
behaviour (mandatory large-screen resizability for apps targeting API 37, app bubbles, bubble bar, interactive picture-in-picture,
Continue On), Compose as the standard UI toolkit, and platform pieces (contact picker, eye dropper, password field
behaviour, Eclipsa HDR). The posts name only the Material 3 Adaptive library; they do not mention blur, theming, dynamic
colour or icons.

What exists about Android 17's look comes from beta observation and press, not from Google: more background blur on
system surfaces in the betas (widget picker, wallpaper blur when an app opens, in later betas the volume panel, power
menu, lock screen shortcuts, notifications), reported as tinted by dynamic colour and compared by the press with Apple's
glass effects ([9to5Google, Beta 3](https://9to5google.com/2026/03/26/android-17-background-blur/), [Android Authority](https://www.androidauthority.com/android-17-system-level-blur-effects-leak-3635131/)).
Google's only statement about the blur is the Android 16 one quoted above. Whether the blur is part of Material 3
Expressive for apps is not published; Android Authority says it is not.

**Consequence for this work.** A look described as Android 17 cannot be taken from a Google source. What was
built is the published Material 3 Expressive system, plus the one visual trait that is documented for Android 16 and
reported for 17 (a blurred, tinted backdrop behind sheets, dialogs and the navigation bar) used with restraint. The
parts of that description that were not confirmed, "bold new design" and "somewhat Apple-like", are not claimed.

## 3. The official web implementations

| Package | State at the time of reading | Licence |
|---|---|---|
| [`@material/web`](https://github.com/material-components/material-web) 2.5.0 (released 2026-07-15) | "In maintenance mode pending new maintainers" ([discussion 5642](https://github.com/material-components/material-web/discussions/5642)): the engineers were reassigned in 2024, no new features or components are planned, pull requests are not accepted by default, releases continue (2.4.0 in Aug 2025, 2.4.1 in Oct 2025, 2.5.0 in Jul 2026, nightly builds in Oct 2026). The maintainer wrote on 19 May 2025 that Expressive features are not planned, and on 20 May 2025 that new design tokens could be generated but updating the components to use them would be much harder. The published package contains no Expressive tokens or components (searched). It has switch, slider, text fields, select, menu, dialog, checkbox, radio, chips, progress, tabs, list, icon button, FAB; in `labs` card, navigation bar and drawer, segmented button, badge. No button group, no loading indicator, no navigation rail, no toolbar. | Apache-2.0 |
| [`@material/material-color-utilities`](https://github.com/material-foundation/material-color-utilities) 0.4.0 | HCT colour space ("based on CAM16 x L*"), dynamic colour, schemes (tonal spot, vibrant, expressive, neutral, fidelity, content, rainbow, fruit salad, monochrome), image quantisation to a source colour, harmonisation. Version 0.4.0 adds a `2025` colour specification (with "dim" roles). | Apache-2.0 |
| [mdui](https://github.com/zdhxiong/mdui) v2 | Material 3 web components (not Expressive); the maintainer is building v3 on the Expressive specification, no release was confirmed. | MIT |
| Roboto Flex ([source](https://github.com/googlefonts/roboto-flex)) | Variable font with weight, width, optical size and more axes. | SIL OFL 1.1 |
| Material Symbols | Variable icon set; the Rounded weight 400 SVG set was used. | Apache-2.0 |

## 4. Decision: one shared package, the real parts where they exist

A sibling package, `foundation-ui` (a sibling checkout), is the single source of tokens and components for both the
control panel (Vue 3 under Tauri's WebView2, with Element Plus) and the Web UI (Vue 3 with Bootstrap).

- **Colour: the real library.** `@material/material-color-utilities` 0.4.0 generates the roles. One seed per theme:
  the existing brand blue `#4a9eff` for light (vibrant scheme, so the blue stays blue) and the existing rose `#d4a5a5` for
  dark (tonal spot). The `2025` colour spec is used. Status colours (success, warning) are harmonised towards the seed and
  kept calm. The wallpaper, when a person sets one, supplies the seed (the library's image quantiser), so the anime
  background keeps working as Material You wallpaper colour; it sits under a scrim of the surface colour, and it is off by
  default because art behind tonal surfaces fights both legibility and the generated hue. The default remote picture is
  no longer loaded unless `wallpaperDefault` is set.
- **Components: real `@material/web` elements** for switch, slider, text field, select and option, checkbox, menu. They
  take the generated `--md-sys-color-*` tokens directly. They are Material 3, not Expressive; their shapes are tuned
  through their documented properties (slider track and handle sizes, field corner radius).
- **Expressive parts that Material Web lacks, written in the package from the published values:** shape scale (all steps
  above), spring motion (the Expressive spring table turned into CSS `linear()` easing at build time, a technique
  documented on MDN; supported in Chromium 113 and later, so in WebView2), emphasized type with Roboto Flex, connected
  button group with shape morph on press and selection, grouped lists with first / middle / last corner shapes, a
  morphing loading indicator (48-point polygons interpolated by `clip-path`), navigation rail (expanded and compact) with
  a bottom bar when narrow, a hero container, dialog (native `<dialog>`, spring entrance), snackbar, save bar.
  The replacement for the unmaintained library is therefore not another library: mdui v2 has the same Material 3
  limits and v3 is unreleased; the missing parts are small and are owned here.
- **Legacy controls** that are still used by pages that were not rebuilt (Element Plus in the panel, Bootstrap in the
  Web UI) are re-skinned with the same tokens in `legacy.css`, so they match without changing behaviour.
- **Dependencies added** (project dependencies of the package, installed on the build host only): `@material/web`,
  `@material/material-color-utilities`, and for building `@material-symbols/svg-400`, `@fontsource-variable/roboto-flex`,
  `vite`, `@vitejs/plugin-vue`. The consumers resolve the package by an alias to the sibling checkout (`FOUNDATION_UI_DIR`);
  declaring it as a `file:` dependency and regenerating the lock files is the remaining packaging step.
- **Licence.** The package is MPL-2.0 (`LICENSE` in the foundation-ui repository); MPL-2.0 section 3.3 lets this repository distribute it together with its own code. Every dependency is Apache-2.0, MIT or OFL 1.1, which are
  compatible; the font and the icon files keep their own licence notices in the package.

## 5. What could not be confirmed

- Exact component specifications of the m3.material.io pages (they are scripted pages the fetch tool could not read):
  sizes, spacing and state-layer values of the expressive navigation rail, lists and buttons were derived from the
  Android implementation documents above and from the Material 3 baseline, not read from the specification pages.
- Line heights and letter spacing of the type scale (baseline values used).
- Whether Android 17 changes the visual design at all beyond blur: no Google source.
- Shape morphing between Material's polygon family (35 presets) as a web library: none was found, so morphing is done
  with CSS `border-radius` and `clip-path` interpolation.
