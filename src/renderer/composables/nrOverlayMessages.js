const messages = {
  "zh": {
    "states": {
      "disabled": "NR 已关闭",
      "active": "NR 已开启",
      "warming_up": "NR 启动中",
      "stopping": "NR 关闭中",
      "scaling": "正在应用设置",
      "degraded": "NR 已降级",
      "idle": "等待串流",
      "no_pipeline": "串流已连接，当前没有可用的 NR 处理管线",
      "choose_stream": "请选择串流",
      "app_disabled": "此应用已关闭 NR",
      "blocked": "NR 不可用",
      "unavailable": "主机未连接"
    },
    "enabledIdle": "设置已保存，下次支持 NR 的串流将尝试开启增强。",
    "enabledNoPipeline": "当前串流没有可用的 NR 管线，设置已保存。下次支持 NR 的串流会按此设置启动。",
    "scale": "NR 处理比例",
    "intensity": "增强强度",
    "advanced": "高级设置",
    "style": "风格",
    "skinStructure": "肤质结构强度",
    "autoMask": "自动遮罩",
    "dvWaiting": "当前未输出 DV 元数据",
    "dvFallback": "DV 未生效，已回退至基础画面",
    "transfer": "传递曲线",
    "uiCorrection": "UI 修正",
    "motionQuality": "运动质量",
    "motionOptions": ["关闭", "性能", "均衡", "质量"],
    "applyHint": "松手后应用；切换时可能短暂停顿",
    "previewSize": "目标处理尺寸",
    "updateHost": "更新主机后可调节比例与高级参数",
    "processing": "处理尺寸",
    "targetSize": "预设尺寸",
    "scaleHint": "降低比例可减少开销；原画尺寸保持不变",
    "settingsFailed": "设置未能生效，已请求恢复上次有效设置",
    "choose": "选择要控制的串流",
    "reselect": "选择当前串流",
    "stream": "串流",
    "drag": "拖动浮层",
    "session": "当前串流",
    "close": "隐藏浮层",
    "enhancement": "DLSS NR（全局）",
    "preserved": "保持输出",
    "remembered": "参数调整会记住，供之后的串流使用",
    "saved": "参数已记住",
    "resetDefaults": "恢复 NR 默认值",
    "saveFailed": "当前串流已调整，但未能保存为下次串流的默认值",
    "opacity": "背景不透明度",
    "shortcut": "切换画面增强",
    "shortcutUnavailable": "快捷键不可用，可点击开关",
    "hdrBusy": "RTX HDR 正在占用增强位置",
    "unsupported": "当前主机或捕获路径不支持实时切换",
    "fallback": "增强未生效，正在使用原画；关闭后可重新尝试",
    "componentUnavailable": "DLSS NR 组件暂不可用。请在画质增强页检查组件状态。",
    "wait": "等待主机处理下一帧；首次开启需要初始化",
    "retry": "重试",
    "disconnected": "无法读取主机状态，请确认 Sunshine 正在运行",
    "ended": "该串流已结束",
    "failed": "切换失败，请重试"
  },
  "en": {
    "states": {
      "disabled": "NR off",
      "active": "NR on",
      "warming_up": "NR starting",
      "stopping": "NR stopping",
      "scaling": "Applying settings",
      "degraded": "NR bypassed",
      "idle": "Waiting for stream",
      "no_pipeline": "Stream connected · no NR pipeline is available for this stream",
      "choose_stream": "Select a stream",
      "app_disabled": "NR disabled for this app",
      "blocked": "NR unavailable",
      "unavailable": "Host disconnected"
    },
    "enabledIdle": "Setting saved. The next supported stream will try to start NR.",
    "enabledNoPipeline": "No NR pipeline is available for this stream. Your setting is saved, and the next supported stream will follow it.",
    "scale": "NR processing scale",
    "intensity": "Enhancement strength",
    "advanced": "Advanced settings",
    "style": "Style",
    "skinStructure": "Skin structure strength",
    "autoMask": "Automatic mask",
    "dvWaiting": "No DV RPU metadata in the current output",
    "dvFallback": "DV inactive; using the base layer",
    "transfer": "Transfer function",
    "uiCorrection": "UI correction",
    "motionQuality": "Motion quality",
    "motionOptions": ["Off", "Fast", "Balanced", "Quality"],
    "applyHint": "Apply on release. Changes may briefly pause processing.",
    "previewSize": "Target processing size",
    "updateHost": "Update the host to adjust scale and advanced controls.",
    "processing": "Processing size",
    "targetSize": "Planned size",
    "scaleHint": "Lower scales reduce cost. Original dimensions stay unchanged.",
    "settingsFailed": "Settings failed; restoration of the previous settings was requested.",
    "choose": "Select a stream to control",
    "reselect": "Select current stream",
    "stream": "Stream",
    "drag": "Drag overlay",
    "session": "Current stream",
    "close": "Hide overlay",
    "enhancement": "DLSS NR (global)",
    "preserved": "Output preserved",
    "remembered": "Parameter changes are remembered for future streams.",
    "saved": "Parameters saved.",
    "resetDefaults": "Reset NR defaults",
    "saveFailed": "This stream changed, but the defaults could not be saved for future streams.",
    "opacity": "Background opacity",
    "shortcut": "Toggle enhancement",
    "shortcutUnavailable": "Shortcut unavailable; use the switch",
    "hdrBusy": "RTX HDR is using the enhancement slot",
    "unsupported": "This host or capture path does not support live switching",
    "fallback": "Using the original image. Switch off and on to retry.",
    "componentUnavailable": "The DLSS NR component is unavailable. Check its status under Image enhancement.",
    "wait": "Waiting for the next frame. First use needs initialization.",
    "retry": "Retry",
    "disconnected": "Cannot read host status. Check that Sunshine is running.",
    "ended": "This stream has ended",
    "failed": "Could not switch. Try again."
  }
}

export function nrOverlayText(locale) {
  return messages[locale.startsWith("zh") ? "zh" : "en"]
}

export function nrStatusLabel(text, state, globalEnabled) {
  if (globalEnabled && state === 'idle') return text.enabledIdle
  if (globalEnabled && state === 'no_pipeline') return text.enabledNoPipeline
  return text.states[state]
}

/** The degraded notice for a pipeline: a known reason gets actionable wording, anything else keeps the fallback plus its raw code. */
export function nrDegradedReason(pipeline, text) {
  const reason = pipeline?.nr_reason || ""
  if (reason === "component_unavailable")
    return { main: text.componentUnavailable, detail: "" }
  return { main: text.fallback, detail: reason }
}
