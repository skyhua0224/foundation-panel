<script setup>
import { computed, onMounted, ref, unref, watch } from 'vue'
import { useNotify, MButton, MDialog, MExpertItem, MFooter, MGroup, MIcon, MIconButton, MItem, MPathItem, MSection, MSeg, MSheet, MSnackbar, MSwitchItem, MSwitch, useAppearance } from 'foundation-ui/vue'
import { useI18n } from '../../desktop/i18n/index.js'
import { defaultDesktopSettings, desktopSettings, loadDesktopSettings, saveDesktopSettings } from '../../desktop/composables/useDesktopSettings.js'
import { bigScreenSettings, defaultBigScreenSettings, setBigScreenSetting } from '../../desktop/composables/useBigScreenSettings.js'
import { useLaunchHelpers } from '../../desktop/composables/useLaunchHelpers.js'
import { useDesktopPet } from '../../composables/useDesktopPet.js'
import { INTERVAL_PRESETS, JITTER_PRESETS, VISION_INTERVAL_PRESETS, usePetSpeech } from '../../composables/usePetSpeech.js'
import { usePanelPart } from '../../composables/panelParts.js'
import { isTauriRuntime } from '../../desktop/composables/useTauri.js'
import { openBigScreenWithNotice } from '../../composables/openBigScreen.js'
import { fillHint } from '../../desktop/composables/padGlyphs.js'
import { padType } from '../../desktop/composables/padType.js'
import { fileMapping, sunshine } from '../../tauri-adapter.js'

// Every option of the earlier desktop window, in the settings of the control panel: start up, notifications, the big screen
// mode, the launch helpers (paths), shared folders, the desktop pet and, behind 高级, the developer and log options. Nothing
// here needs the old window any more. A change is saved at once (the old window had a save button).
const props = defineProps({ locale: { type: String, default: 'zh' } })
const { t, locale, toggleLocale } = useI18n()
const zh = computed(() => String(props.locale).startsWith('zh'))
const st = computed(() => t.value.settings)
const settings = desktopSettings
const hasTauri = ref(false)
const notice = ref('')
const openDesktop = () => openBigScreenWithNotice((m) => { notice.value = m }, t.value.sidebar.openBigScreenFailed)
// short results go to the one snackbar of the window
watch(notice, (message) => { if (message) { useNotify().snack(message); notice.value = '' } })
const sheet = ref('')
const open = (id) => { sheet.value = id }
// how many switches of a group are on, for the row that opens it (`{n} / {total}`)
const onCount = (rows, source) => { const o = unref(source) || {}; return `${rows.filter(([key]) => Boolean(o[key])).length} / ${rows.length}` }
const bigOn = computed(() => bigScreen.value.filter(([key]) => Boolean(unref(bigScreenSettings)?.[key])).length + (autoLaunchDesktop.value ? 1 : 0))
const closeSheet = (value) => { if (!value) sheet.value = '' }
// the toolbar menu's 桌宠设置 opens the pet sheet here
usePanelPart('pet', () => open('pet'))
// and the rows 在 Panel 中打开 of the big screen: tool paths, file sharing, the advanced switches
usePanelPart('helpers', () => open('helpers'))
usePanelPart('sharing', () => openSharing())
usePanelPart('expert', () => open('expert'))

onMounted(async () => {
  hasTauri.value = await isTauriRuntime()
  loadDesktopLaunch()
  await loadDesktopSettings()
  if (sheet.value === 'sharing') loadMappings()
})

async function set(key, value) {
  try { await saveDesktopSettings({ ...settings.value, [key]: value }) } catch (e) { notice.value = `${st.value.updateError || ''} ${e?.message || ''}`.trim() }
}
const startup = computed(() => [['autoStart', st.value.autoStart, st.value.autoStartDesc], ['startMinimized', st.value.startMinimized, st.value.startMinimizedDesc], ['autoStartSunshine', st.value.autoStartSunshine, st.value.autoStartSunshineDesc]])
const notifications = computed(() => [['notifications', st.value.desktopNotifications, st.value.desktopNotificationsDesc], ['connectionNotify', st.value.connectionNotify, st.value.connectionNotifyDesc], ['updateNotify', st.value.updateNotify, st.value.updateNotifyDesc]])
const bigScreen = computed(() => [['autoYieldOnLaunch', st.value.autoYield, st.value.autoYieldDesc], ['gamepadCursorEnabled', st.value.gamepadCursor, fillHint(st.value.gamepadCursorDesc, padType.value)], ['oskAutoOpen', st.value.oskAutoOpen, fillHint(st.value.oskAutoOpenDesc, padType.value)], ['navSounds', st.value.navSounds, st.value.navSoundsDesc]])

// ---- open the Desktop UI when a client starts the Desktop application (an entry of its `detached` commands in apps.json) ----
const GUI_DESKTOP_CMD = '.\\assets\\gui\\sunshine-gui.exe --desktop'
const isDesktopApp = (app) => ['Desktop', '桌面'].includes(app.name)
const isGuiCmd = (cmd) => cmd.includes('sunshine-gui') && (cmd.includes('--desktop') || cmd.includes('-d'))
const autoLaunchDesktop = ref(false)
const hostJson = async (path, init) => { const base = await sunshine.getProxyUrl(); return (await fetch(`${base}${path}`, init)).json() }
async function loadDesktopLaunch() {
  try {
    const resp = await hostJson('/api/apps')
    const app = (resp.apps || resp || []).find(isDesktopApp)
    autoLaunchDesktop.value = !!app && (app.detached || []).some(isGuiCmd)
  } catch { /* the host is not reachable: the switch stays off */ }
}
async function setDesktopLaunch(on) {
  const before = autoLaunchDesktop.value
  autoLaunchDesktop.value = on
  try {
    const resp = await hostJson('/api/apps')
    const apps = resp.apps || resp || []
    const index = apps.findIndex(isDesktopApp)
    if (index < 0) throw new Error('Desktop')
    const detached = (apps[index].detached || []).filter((cmd) => !isGuiCmd(cmd))
    if (on) detached.push(GUI_DESKTOP_CMD)
    const result = await hostJson('/api/apps', { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ apps, editApp: { ...apps[index], detached, index } }) })
    if (String(result.status) !== 'true') throw new Error(result.error || '')
  } catch (e) { autoLaunchDesktop.value = before; notice.value = `${t.value.stream?.msg?.launchModeSaveFailed || 'Could not save'} ${e?.message || ''}`.trim() }
}

// ---- launch helpers: the path of each tool ----
const { templates, getGlobalPath, setGlobalPath } = useLaunchHelpers(t)
const helpers = computed(() => templates.value.filter((x) => x.id !== 'custom' && x.params?.some((p) => p.key === 'path')))
const browseTool = (id) => async () => {
  try {
    const { open: pick } = await import('@tauri-apps/plugin-dialog')
    const path = await pick({ filters: [{ name: t.value.launchHelper?.executableFiles || 'Executables', extensions: ['exe', 'bat', 'cmd', 'lnk', 'com', 'scr'] }, { name: t.value.launchHelper?.allFiles || 'All files', extensions: ['*'] }] })
    return typeof path === 'string' ? path : ''
  } catch { return '' }
}

// ---- shared folders ----
const fs = computed(() => t.value.fileSharing)
const mappings = ref([])
const sharing = ref({ loading: false, busy: false, error: '' })
const friendly = (e) => String(e?.message || e || fs.value.operationFailed)
async function loadMappings() {
  if (!hasTauri.value) { mappings.value = []; return }
  sharing.value.loading = true; sharing.value.error = ''
  try { mappings.value = await fileMapping.list() } catch (e) { mappings.value = []; sharing.value.error = friendly(e) } finally { sharing.value.loading = false }
}
async function addFolder() {
  sharing.value.busy = true; sharing.value.error = ''
  try {
    const { open: pick } = await import('@tauri-apps/plugin-dialog')
    const path = await pick({ directory: true, multiple: false })
    if (!path) return
    const m = await fileMapping.quickShareFolder(path)
    await loadMappings()
    notice.value = fs.value.sharedSuccess.replace('{name}', m?.name || String(path).split(/[\\/]/).pop())
  } catch (e) { sharing.value.error = friendly(e) } finally { sharing.value.busy = false }
}
const revoking = ref(null)
async function revoke() {
  const m = revoking.value; revoking.value = null
  if (!m?.id) return
  sharing.value.busy = true
  try { await fileMapping.remove(m.id); mappings.value = mappings.value.filter((x) => x.id !== m.id); notice.value = fs.value.revokedSuccess.replace('{name}', m.name || m.id) } catch (e) { sharing.value.error = friendly(e) } finally { sharing.value.busy = false }
}
async function menu(on) {
  sharing.value.busy = true
  try { on ? await fileMapping.installMenu() : await fileMapping.uninstallMenu(); notice.value = on ? fs.value.menuInstalled : fs.value.menuRemoved } catch (e) { sharing.value.error = friendly(e) } finally { sharing.value.busy = false }
}
const openSharing = () => { open('sharing'); loadMappings() }

// ---- desktop pet ----
const { petEnabled, isObserving, observeInterval, pokeFailed, startObserving, stopObserving, setIntervalSeconds, poke } = useDesktopPet()
const consent = ref(false)
// the speech of the pet (what the old pet window had besides the desktop observation)
const speech = usePetSpeech()
const speechSeconds = computed(() => INTERVAL_PRESETS.map((v) => ({ value: v, label: v >= 60 ? `${v / 60} ${t.value.petTool.unitMin}` : `${v} ${t.value.petTool.unitSec}` })))
const jitterOptions = computed(() => JITTER_PRESETS.map((v) => ({ value: v, label: `${v}%` })))
const shortcutText = computed(() => (speech.shortcut.value.error ? t.value.petTool.toolbarShortcutStateError : speech.shortcut.value.enabled && !speech.shortcut.value.registered && !speech.shortcut.value.pending ? t.value.petTool.toolbarShortcutUnavailable : t.value.petTool.toolbarShortcutDesc))
const historyTime = (ts) => { const d = new Date(ts); return Number.isNaN(d.getTime()) ? '' : `${String(d.getHours()).padStart(2, '0')}:${String(d.getMinutes()).padStart(2, '0')}` }
const petSeconds = computed({ get: () => Math.round(observeInterval.value / 1000), set: (v) => setIntervalSeconds(Number(v)) })
const intervals = computed(() => VISION_INTERVAL_PRESETS.map((value) => ({ value, label: value >= 60 ? `${value / 60} ${t.value.petTool.unitMin}` : `${value} ${t.value.petTool.unitSec}` })))
const togglePet = (on) => { if (!on) stopObserving(); else consent.value = true }
const acceptPet = () => { consent.value = false; startObserving() }

// ---- appearance: export and import (what the old theme editor offered as theme export and import) ----
const ap = useAppearance()
const picker = ref(null)
const KEYS = ['background', 'source', 'url', 'color', 'preset', 'custom', 'mode', 'dim', 'blur']
function exportAppearance() {
  const data = Object.fromEntries(KEYS.map((k) => [k, ap.state[k]]).filter(([, v]) => v !== undefined))
  const a = document.createElement('a')
  a.href = URL.createObjectURL(new Blob([JSON.stringify(data, null, 2)], { type: 'application/json' }))
  a.download = 'foundation-appearance.json'
  a.click()
  setTimeout(() => URL.revokeObjectURL(a.href), 1000)
}
async function importAppearance(event) {
  const file = event.target.files?.[0]
  event.target.value = ''
  if (!file) return
  try {
    const data = JSON.parse(await file.text())
    await ap.set(Object.fromEntries(KEYS.filter((k) => k in data).map((k) => [k, data[k]])))
  } catch (e) { notice.value = `${t.value.themeEditor?.importFailed || 'Import failed'} ${e?.message || ''}`.trim() }
}

// ---- 高级 (expert) ----
const EXPERT = {
  zh: { changed: '已修改', desc: '作用', effect: '影响', default: '默认值', reset: '恢复默认', on: '开', off: '关', devMode: '打开开发者功能（例如调试输出）；日常使用保持关闭。', textContext: '允许把选中的文字作为上下文交给助手。', logLevel: '面板写入日志的最低级别；级别越低日志越大。' },
  zh_TW: { changed: '已變更', desc: '作用', effect: '影響', default: '預設值', reset: '恢復預設', on: '開', off: '關', devMode: '開啟開發者功能（例如除錯輸出）；日常使用保持關閉。', textContext: '允許把選取的文字作為上下文交給助手。', logLevel: '面板寫入日誌的最低級別；級別越低日誌越大。' },
  en: { changed: 'Changed', desc: 'What it does', effect: 'Effect', default: 'Default', reset: 'Reset to default', on: 'On', off: 'Off', devMode: 'Turns on developer features such as debug output; leave off for daily use.', textContext: 'Lets selected text be handed to the assistant as context.', logLevel: 'The least severe message the panel writes to its log; lower levels make the log larger.' },
  de: { changed: 'Geändert', desc: 'Was es tut', effect: 'Wirkung', default: 'Standard', reset: 'Zurücksetzen', on: 'Ein', off: 'Aus', devMode: 'Schaltet Entwicklerfunktionen wie die Debug-Ausgabe ein; im Alltag aus lassen.', textContext: 'Ausgewählter Text darf als Kontext an den Assistenten.', logLevel: 'Die niedrigste Stufe, die das Panel schreibt; niedrigere Stufen machen den Log größer.' },
  fr: { changed: 'Modifié', desc: 'Ce que cela fait', effect: 'Effet', default: 'Défaut', reset: 'Réinitialiser', on: 'Oui', off: 'Non', devMode: 'Active les fonctions développeur, comme la sortie de débogage ; laissez désactivé au quotidien.', textContext: 'Le texte sélectionné peut servir de contexte à l’assistant.', logLevel: 'Le message le moins grave que le panneau écrit ; un niveau bas agrandit le journal.' },
  es: { changed: 'Cambiado', desc: 'Qué hace', effect: 'Efecto', default: 'Predeterminado', reset: 'Restablecer', on: 'Sí', off: 'No', devMode: 'Activa funciones de desarrollador, como la salida de depuración; déjelo desactivado a diario.', textContext: 'El texto seleccionado puede darse al asistente como contexto.', logLevel: 'El mensaje menos grave que escribe el panel; un nivel bajo agranda el registro.' },
  it: { changed: 'Modificato', desc: 'Che cosa fa', effect: 'Effetto', default: 'Predefinito', reset: 'Ripristina', on: 'Sì', off: 'No', devMode: 'Attiva le funzioni sviluppatore, come l’output di debug; lascialo spento ogni giorno.', textContext: 'Il testo selezionato può andare all’assistente come contesto.', logLevel: 'Il messaggio meno grave che il pannello scrive; un livello basso ingrandisce il registro.' },
  ja: { changed: '変更済み', desc: '何をするか', effect: '影響', default: '既定', reset: '既定に戻す', on: 'オン', off: 'オフ', devMode: 'デバッグ出力などの開発者機能をオンにします。普段はオフのまま。', textContext: '選択した文字を助手の文脈として渡せます。', logLevel: 'パネルがログに書く最も低いレベル。低いほどログは大きくなります。' },
  ko: { changed: '변경됨', desc: '하는 일', effect: '영향', default: '기본값', reset: '기본값으로', on: '켜짐', off: '꺼짐', devMode: '디버그 출력 같은 개발자 기능을 켭니다. 평소에는 끄세요.', textContext: '선택한 글을 도우미의 맥락으로 넘길 수 있습니다.', logLevel: '패널이 로그에 쓰는 가장 낮은 수준. 낮을수록 로그가 커집니다.' },
  ru: { changed: 'Изменено', desc: 'Что делает', effect: 'Эффект', default: 'По умолчанию', reset: 'Сбросить', on: 'Вкл', off: 'Выкл', devMode: 'Включает функции разработчика, например отладочный вывод; в обычной работе выключено.', textContext: 'Выделенный текст можно передать помощнику как контекст.', logLevel: 'Наименее серьёзное сообщение, которое панель пишет в журнал; ниже уровень — больше журнал.' },
  pt_BR: { changed: 'Alterado', desc: 'O que faz', effect: 'Efeito', default: 'Padrão', reset: 'Redefinir', on: 'Ligado', off: 'Desligado', devMode: 'Liga recursos de desenvolvedor, como a saída de depuração; deixe desligado no dia a dia.', textContext: 'O texto selecionado pode ir ao assistente como contexto.', logLevel: 'A mensagem menos grave que o painel grava; níveis mais baixos aumentam o registro.' },
  pt: { changed: 'Alterado', desc: 'O que faz', effect: 'Efeito', default: 'Predefinição', reset: 'Repor', on: 'Ligado', off: 'Desligado', devMode: 'Liga funções de programador, como a saída de depuração; deixe desligado no dia a dia.', textContext: 'O texto selecionado pode ir para o assistente como contexto.', logLevel: 'A mensagem menos grave que o painel escreve; níveis mais baixos aumentam o registo.' },
  bg: { changed: 'Променено', desc: 'Какво прави', effect: 'Ефект', default: 'По подразбиране', reset: 'Нулиране', on: 'Вкл', off: 'Изкл', devMode: 'Включва функции за разработчик, например отладочен изход; в ежедневието остава изключено.', textContext: 'Избраният текст може да отиде при асистента като контекст.', logLevel: 'Най-лекото съобщение, което панелът пише; по-ниско ниво прави журнала по-голям.' },
  cs: { changed: 'Změněno', desc: 'Co dělá', effect: 'Účinek', default: 'Výchozí', reset: 'Obnovit', on: 'Zap', off: 'Vyp', devMode: 'Zapne vývojářské funkce, třeba ladící výstup; v běžném provozu nechte vypnuté.', textContext: 'Vybraný text může jít asistentovi jako kontext.', logLevel: 'Nejméně závažná zpráva, kterou panel zapíše; nižší úroveň zvětší protokol.' },
  pl: { changed: 'Zmieniono', desc: 'Co robi', effect: 'Efekt', default: 'Domyślne', reset: 'Przywróć', on: 'Wł.', off: 'Wył.', devMode: 'Włącza funkcje programisty, na przykład wyjście debug; na co dzień zostaw wyłączone.', textContext: 'Zaznaczony tekst może trafić do asystenta jako kontekst.', logLevel: 'Najmniej poważny komunikat, jaki panel zapisuje; niższy poziom powiększa dziennik.' },
  sv: { changed: 'Ändrad', desc: 'Vad den gör', effect: 'Effekt', default: 'Standard', reset: 'Återställ', on: 'På', off: 'Av', devMode: 'Slår på utvecklarfunktioner som felsökningsutdata; lämna av i vardagen.', textContext: 'Markerad text kan ges till assistenten som sammanhang.', logLevel: 'Det minst allvarliga meddelandet panelen skriver; lägre nivå gör loggen större.' },
  tr: { changed: 'Değişti', desc: 'Ne yapar', effect: 'Etki', default: 'Varsayılan', reset: 'Sıfırla', on: 'Açık', off: 'Kapalı', devMode: 'Hata ayıklama çıktısı gibi geliştirici özelliklerini açar; günlük kullanımda kapalı kalsın.', textContext: 'Seçilen yazı yardımcıya bağlam olarak verilebilir.', logLevel: 'Panelin günlüğe yazdığı en düşük düzey; düzey düştükçe günlük büyür.' },
  uk: { changed: 'Змінено', desc: 'Що робить', effect: 'Ефект', default: 'Типово', reset: 'Скинути', on: 'Увімк', off: 'Вимк', devMode: 'Вмикає функції розробника, наприклад налагоджувальний вивід; у повсякденні лишайте вимкненим.', textContext: 'Виділений текст можна передати помічнику як контекст.', logLevel: 'Найменш серйозне повідомлення, яке панель пише в журнал; нижчий рівень збільшує журнал.' },
}
const expertCode = (raw) => {
  const tag = String(raw || 'en').replace(/-/g, '_')
  if (/^zh/i.test(tag) && /(_TW|_HK|_MO|Hant)/i.test(tag)) return 'zh_TW'
  if (/^zh/i.test(tag)) return 'zh'
  const primary = tag.split('_')[0]
  return EXPERT[tag] ? tag : (EXPERT[primary] ? primary : 'en')
}
const X = computed(() => EXPERT[expertCode(props.locale)] || EXPERT.en)
const logOptions = computed(() => ['error', 'warn', 'info', 'debug'].map((value) => ({ value, label: value.toUpperCase() })))
const reset = (key) => set(key, defaultDesktopSettings[key])
const resetAll = async () => {
  try { await saveDesktopSettings({ ...defaultDesktopSettings }) } catch { /* the notice below */ }
  for (const [k, v] of Object.entries(defaultBigScreenSettings)) setBigScreenSetting(k, v)
  notice.value = st.value.resetSuccess
}
</script>

<template>
  <MSection :title="st.appearance">
    <MGroup>
      <MItem action icon="download" :title="t.themeEditor.exportTheme" data-panel-option="themeExport" @click="exportAppearance" />
      <MItem action icon="upload_file" :title="t.themeEditor.importTheme" data-panel-option="themeImport" @click="picker.click()" />
    </MGroup>
    <input ref="picker" type="file" accept="application/json,.json" hidden @change="importAppearance" />
  </MSection>
  <MSection>
    <MGroup>
      <MItem action icon="power_settings_new" :title="st.startup" data-panel-option="startup" @click="open('startup')"><template #trail><span class="m-item__count">{{ onCount(startup, settings) }}</span></template></MItem>
      <MItem action icon="notifications" :title="st.notifications" data-panel-option="notifications" @click="open('notifications')"><template #trail><span class="m-item__count">{{ onCount(notifications, settings) }}</span></template></MItem>
      <MItem action icon="tv" :title="t.sidebar.openBigScreen" data-panel-option="openBigScreen" @click="openDesktop"><template #trail><MIcon name="open_in_new" /></template></MItem>
      <MItem action icon="tv_gen" :title="st.bigScreen" data-panel-option="bigScreen" @click="open('bigscreen')"><template #trail><span class="m-item__count">{{ bigOn }} / {{ bigScreen.length + 1 }}</span></template></MItem>
    </MGroup>
  </MSection>
  <MSection>
    <MGroup>
      <MItem action icon="bolt" :title="st.launchAssistant" data-panel-option="launchAssistant" @click="open('helpers')" />
      <MItem action icon="folder_shared" :title="fs.title" data-panel-option="fileSharing" @click="openSharing" />
      <MItem action icon="pets" :title="st.pet" :text="speech.masterEnabled.value ? st.petOn : st.petOff" data-panel-option="pet" @click="open('pet')" />
      <MItem action icon="tune" :title="st.advanced" data-panel-option="advanced" @click="open('expert')" />
    </MGroup>
  </MSection>

  <MSheet :model-value="sheet === 'startup'" :title="st.startup" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MSwitchItem v-for="[key, name, info] in startup" :key="key" :model-value="Boolean(settings[key])" :title="name" :info="info" :data-panel-option="key" @update:model-value="set(key, $event)" />
    </MGroup>
  </MSheet>
  <MSheet :model-value="sheet === 'notifications'" :title="st.notifications" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MSwitchItem v-for="[key, name, info] in notifications" :key="key" :model-value="Boolean(settings[key])" :title="name" :info="info" :disabled="key === 'connectionNotify' && !settings.notifications" :data-panel-option="key" @update:model-value="set(key, $event)" />
    </MGroup>
  </MSheet>
  <MSheet :model-value="sheet === 'bigscreen'" :title="st.bigScreen" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MSwitchItem :model-value="autoLaunchDesktop" :title="t.stream.autoLaunchDesktopUI" :info="t.stream.autoLaunchDesktopUIDesc" data-panel-option="autoLaunchDesktop" @update:model-value="setDesktopLaunch" />
      <MSwitchItem v-for="[key, name, info] in bigScreen" :key="key" :model-value="Boolean(bigScreenSettings[key])" :title="name" :info="info" :data-panel-option="key" @update:model-value="setBigScreenSetting(key, $event)" />
    </MGroup>
  </MSheet>

  <MSheet :model-value="sheet === 'helpers'" :title="st.launchAssistant" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <p class="m-panel-opt__lead">{{ st.launchAssistantDesc }}</p>
    <MGroup>
      <MPathItem v-for="h in helpers" :key="h.id" :label="h.name" :info="h.description" :model-value="getGlobalPath(h.id, 'path') || ''" :placeholder="h.params.find((p) => p.key === 'path')?.placeholder || ''" :browse="hasTauri ? browseTool(h.id) : null" :browse-label="fs.chooseFolder" :data-panel-option="`tool:${h.id}`" @update:model-value="setGlobalPath(h.id, 'path', $event)" />
    </MGroup>
  </MSheet>

  <MSheet :model-value="sheet === 'sharing'" :title="fs.title" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MItem icon="info" :title="fs.status" :text="!hasTauri ? fs.desktopOnly : sharing.loading ? fs.syncing : mappings.length ? fs.shareCount.replace('{count}', mappings.length) : fs.notShared">
        <template #trail><MIconButton icon="refresh" :label="fs.refresh" :disabled="sharing.loading || !hasTauri" @click="loadMappings" /></template>
      </MItem>
      <MItem v-for="m in mappings" :key="m.id" icon="folder" :title="m.name || m.id" :text="m.path">
        <template #trail><span class="m-item__count">{{ m.mode === 'readwrite' ? fs.readWrite : fs.readOnly }}</span><MIconButton icon="delete" :label="fs.revoke" :disabled="sharing.busy" @click="revoking = m" /></template>
      </MItem>
      <MItem action icon="add" :title="fs.sharedFolders" :text="fs.sharedFoldersDesc" :disabled="sharing.busy || !hasTauri" data-panel-option="sharedFolders" @click="addFolder" />
      <MItem icon="link" :title="fs.explorerMenu" :text="fs.explorerMenuDesc" data-panel-option="explorerMenu">
        <template #trail><MButton variant="tonal" size="sm" :disabled="sharing.busy || !hasTauri" @click="menu(true)">{{ fs.enable }}</MButton><MButton variant="text" size="sm" :disabled="sharing.busy || !hasTauri" @click="menu(false)">{{ fs.disable }}</MButton></template>
      </MItem>
    </MGroup>
    <p v-if="sharing.error" class="m-panel-opt__error" role="alert">{{ sharing.error }}</p>
    <MFooter>{{ fs.readOnly }} · {{ fs.pairedOnly }} · {{ fs.blockLinks }}</MFooter>
  </MSheet>

  <MSheet :model-value="sheet === 'pet'" :title="st.pet" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <!-- the pet on or off: the first row of the sheet -->
    <MGroup>
      <MSwitchItem v-model="speech.masterEnabled.value" icon="pets" :title="t.petTool.master" :info="t.petTool.masterDesc" data-panel-option="petMaster" />
      <MItem action icon="language" :title="t.petTool.language" :text="t.petTool.languageDesc" data-panel-option="petLanguage" @click="toggleLocale">
        <template #trail><span class="m-item__count">{{ locale === 'zh' ? 'EN' : '中' }}</span></template>
      </MItem>
    </MGroup>

    <!-- 对话 -->
    <MSection :title="t.petTool.tabs.speech">
      <MGroup>
        <MSwitchItem v-model="speech.randomEnabled.value" icon="forum" :title="t.petTool.random" :info="t.petTool.randomDesc" :disabled="!speech.masterEnabled.value" data-panel-option="petRandom" />
        <MItem v-if="speech.masterEnabled.value && speech.randomEnabled.value" icon="timer" :title="t.petTool.speechInterval" :text="t.petTool.speechIntervalDesc" stack data-panel-option="petSpeechInterval">
          <template #control><MSeg v-model="speech.randomIntervalSec.value" block :label="t.petTool.speechInterval" :options="speechSeconds" /></template>
        </MItem>
        <MItem v-if="speech.masterEnabled.value && speech.randomEnabled.value" icon="swap_vert" :title="t.petTool.jitter" :text="t.petTool.jitterDesc" stack data-panel-option="petJitter">
          <template #control><MSeg v-model="speech.jitterPercent.value" block :label="t.petTool.jitter" :options="jitterOptions" /></template>
        </MItem>
      </MGroup>
    </MSection>

    <!-- 桌面观察 -->
    <MSection :title="t.petTool.vision">
      <MGroup>
        <MSwitchItem :model-value="petEnabled" icon="visibility" :title="st.deskObserve" :info="t.petTool.visionHelp" :disabled="!speech.masterEnabled.value || !speech.aiReady.value" data-panel-option="deskObserve" @update:model-value="togglePet" />
        <MItem v-if="speech.masterEnabled.value && !speech.aiReady.value" icon="warning" :title="t.petTool.aiKeyMissing" :text="t.petTool.aiKeyMissingDesc" data-panel-option="petAiMissing" />
        <MItem v-if="speech.masterEnabled.value && petEnabled" icon="timer" :title="st.observeInterval" :text="petSeconds < 60 ? `${st.observeIntervalDesc} ⚠ ${t.petTool.visionCostHint}` : st.observeIntervalDesc" stack data-panel-option="observeInterval">
          <template #control><MSeg v-model="petSeconds" block :label="st.observeInterval" :options="intervals" /></template>
        </MItem>
        <MItem v-if="speech.masterEnabled.value && petEnabled" action icon="touch_app" :title="st.pokeMita" :text="pokeFailed ? t.petTool.pokeFailed : st.pokeMitaDesc" :disabled="isObserving || !speech.aiReady.value" data-panel-option="pokeMita" @click="poke">
          <template #trail><span class="m-item__count">{{ isObserving ? st.pokeBtnObserving : st.pokeBtn }}</span></template>
        </MItem>
        <MItem action icon="history" :title="t.petTool.visionHistory" :text="t.petTool.visionHistoryDesc" data-panel-option="petHistory" @click="speech.toggleHistory">
          <template #trail><span class="m-item__count">{{ t.petTool.visionHistoryView }}</span></template>
        </MItem>
        <template v-if="speech.historyOpen.value">
          <MItem v-if="!speech.history.value.length" icon="inbox" :title="t.petTool.visionHistoryEmpty" />
          <MItem v-for="(entry, i) in speech.history.value" :key="i" icon="schedule" :title="entry.text" :text="historyTime(entry.timestamp)" />
          <MItem v-if="speech.history.value.length" action icon="delete" :title="t.petTool.visionHistoryClear" data-panel-option="petHistoryClear" @click="speech.clearHistory" />
        </template>
      </MGroup>
      <MFooter>{{ t.petTool.visionDesc }}</MFooter>
    </MSection>

    <!-- 快捷键 -->
    <MSection :title="t.petTool.tabs.shortcuts">
      <MGroup>
        <MSwitchItem :model-value="speech.shortcut.value.enabled" icon="keyboard" :title="t.petTool.toolbarShortcut" :info="t.petTool.toolbarShortcutDesc" :disabled="speech.shortcut.value.pending" data-panel-option="petShortcut" @update:model-value="speech.setShortcut" />
        <MItem v-if="speech.shortcut.value.error" icon="error" :title="t.petTool.toolbarShortcutStateError" data-panel-option="petShortcutError" />
        <MItem v-else-if="speech.shortcut.value.enabled && !speech.shortcut.value.registered && !speech.shortcut.value.pending" icon="warning" :title="t.petTool.toolbarShortcutUnavailable" data-panel-option="petShortcutUnavailable" />
      </MGroup>
    </MSection>

    <MGroup>
      <MItem action icon="restart_alt" :title="t.petTool.reset" :text="t.petTool.resetTooltip" data-panel-option="petReset" @click="speech.resetDefaults" />
    </MGroup>
  </MSheet>
  <MDialog v-model="consent" :title="t.petTool.visionPrivacyConfirm.title" icon="visibility" tone="info">
    <p>{{ t.petTool.visionPrivacyConfirm.message }}</p>
    <template #actions><MButton variant="text" @click="consent = false">{{ t.petTool.visionPrivacyConfirm.cancel }}</MButton><MButton @click="acceptPet">{{ t.petTool.visionPrivacyConfirm.accept }}</MButton></template>
  </MDialog>

  <MSheet :model-value="sheet === 'expert'" :title="st.advanced" side="end" :close-label="fs.close" @update:model-value="closeSheet">
    <MGroup>
      <MExpertItem :title="st.textContextEnabled" :value="settings.textContextEnabled ? X.on : X.off" :changed="Boolean(settings.textContextEnabled) !== defaultDesktopSettings.textContextEnabled" :desc="st.textContextEnabledDesc" :effect="X.textContext" :def-text="defaultDesktopSettings.textContextEnabled ? X.on : X.off" :texts="X" data-panel-option="textContextEnabled" @reset="reset('textContextEnabled')">
        <MSwitch :model-value="Boolean(settings.textContextEnabled)" :label="st.textContextEnabled" @update:model-value="set('textContextEnabled', $event)" />
      </MExpertItem>
      <MExpertItem stack :title="st.logLevel" :value="st.logLevels[settings.logLevel]" :changed="settings.logLevel !== defaultDesktopSettings.logLevel" :desc="st.logLevelDesc" :effect="X.logLevel" :def-text="st.logLevels[defaultDesktopSettings.logLevel]" :texts="X" data-panel-option="logLevel" @reset="reset('logLevel')">
        <MSeg :model-value="settings.logLevel" block :label="st.logLevel" :options="logOptions" @update:model-value="set('logLevel', $event)" />
      </MExpertItem>
    </MGroup>
    <MButton variant="text" icon="restart_alt" @click="resetAll">{{ st.resetDefaults }}</MButton>
  </MSheet>

  <MDialog :model-value="!!revoking" :title="fs.revoke" icon="delete" tone="warning" @update:model-value="(v) => { if (!v) revoking = null }">
    <p>{{ revoking && fs.revokeConfirm.replace('{name}', revoking.name || revoking.id) }}</p>
    <template #actions><MButton variant="text" @click="revoking = null">{{ fs.close }}</MButton><MButton variant="danger" @click="revoke">{{ fs.revoke }}</MButton></template>
  </MDialog>
</template>

<style>
.m-panel-opt__lead { margin: 0 0 12px; font: var(--m-body-medium); color: var(--md-sys-color-on-surface-variant); }
.m-panel-opt__error { margin: 8px 16px 0; font: var(--m-body-small); color: var(--md-sys-color-error); }
</style>
