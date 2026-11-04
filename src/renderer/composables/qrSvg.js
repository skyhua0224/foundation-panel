// The pairing code as an SVG picture, not a PNG: `QRCode.toDataURL` draws a canvas and encodes a PNG (38-47 ms of one task when the 连接设备 dialog opened, measured on the real
// WebView2); the matrix of the same code is a few hundred squares, one path of runs, built in a millisecond and sharp at every size and scale.

/** The SVG text of a QR matrix (`{ size, get(row, col) }`, what `QRCode.create(...).modules` is): black runs on white, a quiet zone of `margin` modules around it. */
export function qrSvg(modules, { margin = 2, dark = '#000000', light = '#ffffff' } = {}) {
  const size = modules.size
  const total = size + margin * 2
  let d = ''
  for (let row = 0; row < size; row += 1) {
    let col = 0
    while (col < size) {
      if (!modules.get(row, col)) { col += 1; continue }
      let end = col
      while (end < size && modules.get(row, end)) end += 1
      d += `M${col + margin} ${row + margin}h${end - col}v1h${col - end}z`
      col = end
    }
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 ${total} ${total}" shape-rendering="crispEdges"><rect width="${total}" height="${total}" fill="${light}"/><path d="${d}" fill="${dark}"/></svg>`
}

/** The same as an image address for an <img>. */
export const qrSvgDataUrl = (modules, options) => `data:image/svg+xml;charset=utf-8,${encodeURIComponent(qrSvg(modules, options))}`
