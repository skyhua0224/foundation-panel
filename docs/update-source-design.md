# Update source and the Mirror酱 CDK: what the native side still has to build

Status 2026-11-03. The renderer side is built (`useUpdateSource.js`, the 软件更新 section of `ControlPanelPage.vue`, `foundation-ui/update`: `source.js`, `mirrorchyan.js`, `sourceTexts.js`, with tests
and a mock transport). The Rust side below is **stubbed**: the renderer calls these commands and treats a missing command as "the secure store needs a newer panel". They are built when the Windows
VM is back, because they cannot be compiled on Linux (`libspa`).

## What the updater already does (read from `src-tauri/src/update.rs`)

- It **does** verify the download: `validate_download_file_identity` checks the size, the SHA-256 against `expected_sha256` and the `MZ` header after the download, and again on the protected
  staging copy right before install (`protected installer verification failed`). `expected_sha256` comes from the release-metadata asset (`download_sha256`); without a valid one the
  install is refused. Channels in the Rust code: `StableOnly`, `StableAndPrerelease` (beta = "the newer of stable and beta"), `PrereleaseOnly`. There is no third user channel.
- It asks `.com` first and `.cn` only on failure (changed 2026-11-03).

## Requirement for the Mirror酱 path

A file downloaded from a Mirror酱 URL is verified against the SHA-256 of **our** release metadata for that version and asset (the `windows-x64-installer` entry), exactly like a CNB or GitHub
download. The `sha256` Mirror酱 sends is not trusted for the install decision. If our metadata has no asset hash for that version, the Mirror酱 download is refused and the CNB/GitHub path is used.

## Commands (all `#[tauri::command]`, registered in the invoke handler)

| command | arguments | result |
| --- | --- | --- |
| `update_cdk_set` | `{ cdk: string }` | `true`; stores the CDK in the OS secure store (Windows Credential Manager, target `foundation-sunshine-panel/mirrorchyan-cdk`, e.g. the `keyring` crate). Never written to a file, never logged. |
| `update_cdk_present` | none | `bool` |
| `update_cdk_clear` | none | `true` |
| `update_mirrorchyan_check` | `{ currentVersion, channel: 'stable' \| 'beta' }` | `{ status: number, json: object \| null }`: the raw HTTP status and body of `GET https://mirrorchyan.com/api/resources/foundation-sunshine/latest?current_version=&cdk=&os=windows&arch=amd64&channel=&user_agent=foundation-sunshine-panel`; the CDK is read from the store here and appended here only. `status: 0` and `json: null` when there was no answer. |
| `set_update_source` | `{ source: 'auto' \| 'cnb' \| 'github' \| 'mirror' }` | `true`; saved in `update_preferences.json` next to `include_prerelease`. |

HTTP: `reqwest` with a 8 s timeout, no redirects to other hosts, the `user_agent` query parameter as above, no cookies; redact `cdk=` in every log line and error text.

## How the source is used

- `auto`: our metadata, CNB first then GitHub (the metadata already carries both URLs per asset).
- `cnb` / `github`: only that host for the download.
- `mirror`: ask Mirror酱 for the version with `update_mirrorchyan_check`. With a CDK that is valid (`code 0` and a `data.url`) the download goes through that `url`; without one, or on any
  error, the version is the answer and the download goes CNB → GitHub from our metadata. The renderer maps the error codes to sentences (`mirrorchyan.js`, `sourceTexts.js`).
- In every case: verify as above before install.
