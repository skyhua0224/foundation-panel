# Memory: adaptive release of idle windows

The Panel keeps the hidden main window resident while system memory is plentiful. Releasing it under pressure can save memory and increases the cost of reopening.

## What is kept and what is released

| Window | Policy |
|---|---|
| Tray menu window (Material menu) | Never released, in any tier, at any pressure. The tray menu is always the Material menu. A click while the window is still being made waits for it. The native menu is only the last resort when WebView2 refuses the window after every attempt (`ctx_popup::Health::Failed`) or the person chose it in the settings. |
| Main window, hidden to the tray | Eligible for release at high or severe measured system pressure if the reopening guard allows it. State is saved before release. |
| Launch splash | Warm at the start in the adaptive tier while memory is plentiful; made on the host's announcement otherwise; released when idle. |

## Tiers and system pressure

* Default: `auto`. The installed memory picks the tier: up to 8.5 GB the low tier, above it the adaptive tier. `SUNSHINE_GUI_MEM_MODE=off|adaptive|low|auto` or `{"mode": "..."}` in `<config>/Sunshine GUI/memory-mode.json` overrides it (`off` releases nothing).
* Windows pressure comes from available physical memory, system commit headroom and the low-memory notification. It is sampled every 3 s. High begins below max(15%, 1.5 GiB) of available physical memory or 2 GiB of commit headroom. Severe begins below max(6%, 512 MiB) or 768 MiB, or when Windows signals low memory. The numeric recovery thresholds have a 25% margin. See `panel-memory-win32-physical`, `panel-memory-win32-commit`, `panel-memory-win32-notification` and `panel-memory-win32-notification-query` in `docs/authority/source-index.tsv`.
* Adaptive and Low keep the hidden main window while pressure is normal or the system sample is unavailable. A fresh high or severe sample makes a hidden window eligible at the next poll. The prewarm grace period still protects an imminent reopen. Simulated pressure and idle delay overrides apply only to splash measurements.

## Reopening and restoring state

1. While the page is used, `inject-viewstate.js` keeps a snapshot (route, scroll positions, selected tabs, open sections, drafts typed by the person; never a password, hidden or file field, nor a field named like a secret, token, PIN, OTP or card) and hands it to the native side (`view_state.rs`). It lives in memory only and is never written to disk. The Web UI pages in the frame report theirs to the shell by `postMessage`.
2. Tray hover or press, opening the tray menu, the toolbar shortcut press, and an activation message from a second launch can begin making the main window again, hidden, with the snapshot in `window.__PANEL_RESTORE__`. The router starts from the saved stack; the rest is applied as the page draws, and stops the moment the person touches the page.
3. A click can show a prewarmed window immediately. If the window is still being created, the click waits for the page to become ready.

## The guard

The controller times an initial cold construction before qualifying a main
window for release. The cutoff is the median kept reopen (at least 100 ms)
plus a quarter or 50 ms, whichever is larger. An unknown or slow cold cost
keeps the main window in memory. A tray click with no advance signal can
arrive at any pressure level. A released window that exceeds the cutoff
blocks further release at that pressure for six hours
(`memory-guard.json`). `SUNSHINE_GUI_MEM_GUARD=off` bypasses the latency
guard for measurement and keeps the real pressure requirement in force.

## One WebView2 environment, and 0x8007139F

`0x8007139F` is `HRESULT_FROM_WIN32(ERROR_INVALID_STATE)`: WebView2 refuses to make a WebView when the options of the new environment differ from the WebViews already running in the browser process of the same user data folder (sources: `docs/authority/source-index.tsv`, `panel-ms-webview2-process-model`, `panel-ms-webview2-env-options-invalid-state`). The panel appended its switches to an inherited `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` on every start, so a panel started by a panel (restart after a crash, the elevated restart, the updater's relaunch) ran with the switches twice while the browser process of its parent still ran with them once. `webview_env.rs` composes the variable idempotently, makes every window from one environment object, retries a window that fails with this code (250, 750, 2000, 5000 ms) and logs the stage, the options in force and the WebView2 processes with their parents on every failure.
