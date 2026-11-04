// The origin a frame can be talked to at. An iframe that shows about:blank (it sleeps behind a page of the panel, or its page has not been asked for yet) has the origin 'null', and
// `postMessage(message, 'null')` throws a SyntaxError: the error surface of the panel opened with 「出错了」 whenever the pending bridge looked at a sleeping frame (every 600 ms).

/** The origin of an iframe element's address, or '' when the frame is not on an address that a message can be sent to (about:blank, nothing, anything that is not http or https). */
export function frameOrigin(el, base = globalThis.location?.href) {
  if (!el?.src) return ''
  try {
    const url = new URL(el.src, base)
    return /^https?:$/.test(url.protocol) ? url.origin : ''
  } catch {
    return ''
  }
}

/** Sends to the window of `el` when there is an address for it; returns whether it was sent. */
export function postToFrame(el, message, base) {
  const origin = frameOrigin(el, base)
  const target = el?.contentWindow
  if (!origin || !target) return false
  target.postMessage(message, origin)
  return true
}
