// Text helpers for the DualSense pages: placeholder filling and the plain-language
// sentence for a failure code. No window or i18n state is touched here.

/** Replaces `{name}` placeholders; unknown placeholders stay visible. */
export function formatText(template, params = {}) {
  return String(template ?? '').replace(/\{(\w+)\}/g, (match, name) =>
    Object.prototype.hasOwnProperty.call(params, name) ? String(params[name]) : match)
}

const CODE = /^(DS5-[A-Z]+-\d{3}):/

export const dualSenseErrorCode = (message) => String(message || '').match(CODE)?.[1] || ''

/** Plain-language sentence for a failure code; never shows the raw message. */
export function dualSenseErrorText(message, errors, fallbackKey) {
  return errors?.[dualSenseErrorCode(message)] || errors?.[fallbackKey] || ''
}

/** Sentence for a save the host refused, by its numeric code. */
export function refusedSaveText(code, refused) {
  return refused?.[String(code)] || refused?.other || ''
}
