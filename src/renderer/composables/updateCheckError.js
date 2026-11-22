// The error text of a manual update check (src-tauri/src/update.rs, PRERELEASE_UNAVAILABLE_ERROR): "prerelease_unavailable: <reason>" means the check found
// nothing newer but could not ask the prerelease side (the metadata prerelease channel was empty and the GitHub fallback failed, e.g. the anonymous rate limit).
// The panel then says that the prerelease information cannot be fetched right now, never "up to date".
const PREFIX = 'prerelease_unavailable'

export const isPrereleaseUnavailable = (error) => {
  const text = typeof error === 'string' ? error : String(error?.message ?? error ?? '')
  return text === PREFIX || text.startsWith(`${PREFIX}:`)
}

/** The sentence for a failed manual check: the prerelease one when that is the reason, the general one otherwise. */
export const updateCheckFailureText = (error, texts) => (isPrereleaseUnavailable(error) ? texts.prereleaseUnavailable : isCurrentVersionUnknown(error) ? texts.currentVersionUnknown : texts.checkUpdatesFailed)

/** Whether a check result offers a newer version: a result with is_latest is the answer "up to date" (the sheet of the newest version), not an update. */
export const hasUpdate = (info) => !!info && !info.is_latest

// "current_version_unknown: <reason>": the installed version could not be read, so nothing could be compared (a check never reports an update it cannot confirm).
export const isCurrentVersionUnknown = (error) => /^current_version_unknown(:|$)/.test(typeof error === 'string' ? error : String(error?.message ?? error ?? ''))
