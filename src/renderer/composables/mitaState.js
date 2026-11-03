import { reactive } from 'vue'

// What the caption bar's Mita button shows, shared by every part of the panel: a reply is being made (`thinking`, a count, requests can overlap), an answer arrived while the
// chat was not open (`unread`), the chat is open (`open`). The chat itself keeps its history; this only says what is going on.
export const mitaState = reactive({ thinking: 0, unread: false, open: false })

export const mitaBegin = () => { mitaState.thinking += 1 }
export function mitaEnd(answered) {
  mitaState.thinking = Math.max(0, mitaState.thinking - 1)
  if (answered && !mitaState.open) mitaState.unread = true
}
export function mitaOpened(open) {
  mitaState.open = open
  if (open) mitaState.unread = false
}

const TEXTS = {
  zh: { label: '米塔 AI', thinking: '正在想', answer: '有新回复' },
  en: { label: 'Mita AI', thinking: 'thinking', answer: 'new reply' },
}
export const mitaTexts = (locale) => (String(locale || '').toLowerCase().startsWith('zh') ? TEXTS.zh : TEXTS.en)
