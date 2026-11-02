// Icon set inside the thick sliders: one glyph per kind of setting, picked from the key name (the Web page uses the same table).
const RULES = [
  [/\.audio\.micLevel$/, 'mic'],
  [/\.audio\./, 'volume_up'],
  [/\.led\./, 'lightbulb'],
  [/\.trigger\.|\.axis\.trigger\./, 'touch_app'],
  [/\.haptics\.|\.rumble\./, 'mobile_vibrate'],
  [/\.gyro\./, 'sensors'],
  [/\.touchpad\./, 'touch_app'],
  [/\.stick\./, 'gamepad'],
  [/\.sync\.|\.compat\./, 'sync'],
]
/** @param {string} key Tuning key name, for example ds5.haptics.hd.gain. */
export function sliderIcon(key) {
  const hit = RULES.find(([pattern]) => pattern.test(`.${key}`))
  return hit ? hit[1] : ''
}
