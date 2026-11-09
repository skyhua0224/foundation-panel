// A save dialog answers with no path when the person closes it. That is a quiet
// no-op, not an error: the caller gets null and writes nothing. A real failure
// (the dialog channel, the write) still rejects with the reason the caller shows.
export const SAVE_CANCELLED = 'cancelled'

/** The command's cancel sentinel, or the DOM's AbortError from a file picker. */
export function isSaveCancelled(error) {
  if (error === null || error === undefined) return false
  if (typeof error === 'string') return error === SAVE_CANCELLED
  return error.name === 'AbortError' || error.message === SAVE_CANCELLED
}

/** Runs a save command; resolves its answer, or null when the person cancelled. */
export async function saveThroughDialog(invoke, command, args) {
  try {
    return await invoke(command, args)
  } catch (error) {
    if (isSaveCancelled(error)) return null
    throw error
  }
}
