// The address of the host's API through the panel's proxy. The page is up before the address is known; a request made with an empty prefix is `/api/...` against the panel's own
// origin, which answers "asset not found" (api/config, api/system/info and favicon.ico in the log of every start). A request waits for the address instead.

/** `current()` the address when it is known ('' before), `fetchAddress()` asks the panel for it. Returns the address without a trailing slash, or throws when there is none. */
export async function hostBase(current, fetchAddress) {
  const known = String((typeof current === 'function' ? current() : current) || '').replace(/\/$/, '')
  if (known) return known
  const asked = String((await fetchAddress?.()) || '').replace(/\/$/, '')
  if (!asked) throw new Error('the address of the host API is not known yet')
  return asked
}

/** A fetch of `path` through the proxy that never goes to the panel's own origin. */
export const hostFetch = (base, path, init) => base().then((prefix) => fetch(`${prefix}${path}`, init))
