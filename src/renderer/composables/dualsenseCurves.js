// Shapes drawn by the curve and trigger previews. They illustrate the setting;
// they are not the host's exact transfer functions.

export function curveShape(curve, value) {
  const m = Math.min(1, Math.max(0, value))
  switch (curve) {
    case 'enhanced':
      return m * (0.5 + 0.5 * m)
    case 'quadratic':
      return m * m
    case 'cubic':
      return m * m * m
    case 'easeOutQuad':
      return 1 - (1 - m) * (1 - m)
    case 'easeOutCubic':
      return 1 - Math.pow(1 - m, 3)
    case 'linear':
    default:
      return m
  }
}

/** Force at a trigger position (0..1) for one preview mode. */
export function triggerForce(mode, position, { enabled = true, strength = 1, sideStrength = 1 } = {}) {
  if (!enabled) return 0
  const scale = Math.max(0, Math.min(2, strength * sideStrength))
  switch (mode) {
    case 'weapon':
      return position < 0.18 ? scale * 0.85 : 0.04
    case 'vibration':
      return 0.5 + 0.45 * scale * Math.sin(position * Math.PI * 14)
    case 'slope':
      return Math.max(0, Math.min(1, (position - 0.15) / 0.7)) * scale
    case 'feedback':
    default:
      if (position < 0.2 || position > 0.8) return 0
      return Math.min(1, scale * 0.9)
  }
}
