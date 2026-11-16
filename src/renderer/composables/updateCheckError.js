// The error text of a manual update check (src-tauri/src/update.rs, PRERELEASE_UNAVAILABLE_ERROR): "prerelease_unavailable: <reason>" means the check found
// nothing newer but could not ask the prerelease side (the metadata prerelease channel was empty and the GitHub fallback failed, e.g. the anonymous rate limit).
// The panel then says that the prerelease information cannot be fetched right now, never "up to date".
const PREFIX = 'prerelease_unavailable'

export const isPrereleaseUnavailable = (error) => {
  const text = typeof error === 'string' ? error : String(error?.message ?? error ?? '')
  return text === PREFIX || text.startsWith(`${PREFIX}:`)
}

/** The sentence for a failed manual check: the prerelease one when that is the reason, the general one otherwise. */
export const updateCheckFailureText = (error, texts) => (isPrereleaseUnavailable(error) ? texts.prereleaseUnavailable : texts.checkUpdatesFailed)
