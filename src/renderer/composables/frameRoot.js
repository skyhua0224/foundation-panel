// The control panel's frame shows Web UI pages. They must wear the panel's colours and its light or dark choice, and the same "a picture is behind
// me" state (the text backings of the shared styles depend on it), without painting the picture themselves: the panel paints it behind the frame.
// The panel sends the custom properties of its root element, minus the picture and its blur, a class list, the colour scheme and the text of the
// theme's style element (the colour roles of the seed).

const PROP = /^--(md-sys-color|md-ref|m-)[a-z0-9-]*$/
const SKIP = /^--m-(wallpaper|wp-)/
const CLASSES = ['m-has-wallpaper', 'dark']

/** The message the panel sends about its root element (any object with `style`, `classList` and `getAttribute`). */
export function rootSnapshot(root) {
  const props = {}
  for (const name of root.style) if (PROP.test(name) && !SKIP.test(name)) props[name] = root.style.getPropertyValue(name)
  const themeCss = root.ownerDocument?.getElementById('foundation-ui-theme')?.textContent || ''
  // list: how the list pane meets the frame (个性化), so a page inside the frame draws its list the same way
  const list = root.getAttribute('data-list') === 'one' ? 'one' : 'merged'
  return { type: 'foundation-root', props, classes: CLASSES.filter((c) => root.classList.contains(c)), theme: root.getAttribute('data-bs-theme') || '', themeCss, list }
}

/**
 * Keeps a frame in step with the root element: the frame gets a new snapshot whenever the root's style, classes or scheme change
 * (a new wallpaper colour, light or dark, the picture on or off). `getFrame` returns the iframe element, `getOrigin` the origin its page has.
 * Returns the current snapshot (the answer to the frame's own request) and a stop function.
 */
export function followRoot(getFrame, getOrigin, root = document.documentElement) {
  let queued = false
  const push = () => {
    if (queued) return
    queued = true
    requestAnimationFrame(() => {
      queued = false
      const target = getFrame()?.contentWindow
      const origin = getOrigin()
      if (target && /^https?:\/\//.test(origin || '')) target.postMessage(rootSnapshot(root), origin) // never 'null': that throws
    })
  }
  const observer = new MutationObserver(push)
  observer.observe(root, { attributes: true, attributeFilter: ['style', 'class', 'data-bs-theme', 'data-list'] })
  if (root.ownerDocument?.head) observer.observe(root.ownerDocument.head, { childList: true, subtree: true, characterData: true })
  return { snapshot: () => rootSnapshot(root), stop: () => observer.disconnect() }
}
