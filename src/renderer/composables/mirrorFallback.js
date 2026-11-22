// "Mirror酱 暂未同步此版本，已改用 CNB 下载": the one sentence for an update that was not taken from Mirror酱 although the person chose it.
// The reasons are the words of src-tauri/src/update/mirrorchyan.rs (Miss::word), which are also what the anonymous usage statistics carry.
export const MIRROR_MISS_REASONS = Object.freeze(['version_lag', 'cdk_invalid', 'quota', 'network', 'no_cdk'])

/** The sentence, or '' while the reason or the source that took over is not known yet (or the reason is not one of the words). */
export function mirrorFallbackText(reason, source, texts) {
  if (!MIRROR_MISS_REASONS.includes(reason) || !source) return ''
  const line = texts?.[reason]
  return typeof line === 'string' ? line.replaceAll('{source}', source) : ''
}
