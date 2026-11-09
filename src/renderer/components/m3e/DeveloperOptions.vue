<template>
  <MGroup>
    <MItem action icon="code" :title="D.title" data-panel-option="developer-page" @click="emit('open')" />
    <MSwitchItem :model-value="Boolean(settings.devMode)" :title="D.debug" :info="D.info" data-panel-option="devMode" @update:model-value="set($event)" />
  </MGroup>
</template>

<script setup>
import { computed } from 'vue'
import { MGroup, MItem, MSwitchItem } from 'foundation-ui/vue'
import { desktopSettings, saveDesktopSettings } from '../../desktop/composables/useDesktopSettings.js'

// What the panel adds to the 开发者 entry of 关于: the debug output of the window. It is only reachable once developer mode is on (seven taps on the version).
const props = defineProps({ locale: { type: String, default: 'zh' } })
const emit = defineEmits(['open'])
const DEV = {
  zh: { title: '开发者选项', debug: '调试输出', info: '打开面板的调试输出（例如开发者工具）；日常使用保持关闭。' },
  zh_TW: { title: '開發者選項', debug: '除錯輸出', info: '開啟面板的除錯輸出（例如開發者工具）；日常使用保持關閉。' },
  en: { title: 'Developer options', debug: 'Debug output', info: 'Turns on the debug output of the panel (such as the developer tools); leave off for daily use.' },
  de: { title: 'Entwickleroptionen', debug: 'Debug-Ausgabe', info: 'Schaltet die Debug-Ausgabe des Panels ein (etwa die Entwicklerwerkzeuge); im Alltag aus lassen.' },
  fr: { title: 'Options développeur', debug: 'Sortie de débogage', info: 'Active la sortie de débogage du panneau (comme les outils de développement) ; laissez désactivé au quotidien.' },
  es: { title: 'Opciones de desarrollador', debug: 'Salida de depuración', info: 'Activa la salida de depuración del panel (como las herramientas de desarrollo); déjelo desactivado a diario.' },
  it: { title: 'Opzioni sviluppatore', debug: 'Output di debug', info: 'Attiva l’output di debug del pannello (come gli strumenti di sviluppo); lascialo spento ogni giorno.' },
  ja: { title: '開発者オプション', debug: 'デバッグ出力', info: 'パネルのデバッグ出力（開発者ツールなど）をオンにします。普段はオフのまま。' },
  ko: { title: '개발자 옵션', debug: '디버그 출력', info: '패널의 디버그 출력(개발자 도구 등)을 켭니다. 평소에는 끄세요.' },
  ru: { title: 'Параметры разработчика', debug: 'Отладочный вывод', info: 'Включает отладочный вывод панели (например инструменты разработчика); в обычной работе выключено.' },
  pt_BR: { title: 'Opções de desenvolvedor', debug: 'Saída de depuração', info: 'Liga a saída de depuração do painel (como as ferramentas de desenvolvedor); deixe desligado no dia a dia.' },
  pt: { title: 'Opções de programador', debug: 'Saída de depuração', info: 'Liga a saída de depuração do painel (como as ferramentas de programador); deixe desligado no dia a dia.' },
  bg: { title: 'Настройки за разработчик', debug: 'Отладочен изход', info: 'Включва отладочния изход на панела (например инструментите); в ежедневието остава изключено.' },
  cs: { title: 'Možnosti vývojáře', debug: 'Ladící výstup', info: 'Zapne ladící výstup panelu (třeba nástroje vývojáře); v běžném provozu nechte vypnuté.' },
  pl: { title: 'Opcje programisty', debug: 'Wyjście debug', info: 'Włącza wyjście debug panelu (na przykład narzędzia programisty); na co dzień zostaw wyłączone.' },
  sv: { title: 'Utvecklaralternativ', debug: 'Felsökningsutdata', info: 'Slår på panelens felsökningsutdata (till exempel utvecklarverktyg); lämna av i vardagen.' },
  tr: { title: 'Geliştirici seçenekleri', debug: 'Hata ayıklama çıktısı', info: 'Panelin hata ayıklama çıktısını (geliştirici araçları gibi) açar; günlük kullanımda kapalı kalsın.' },
  uk: { title: 'Параметри розробника', debug: 'Налагоджувальний вивід', info: 'Вмикає налагоджувальний вивід панелі (наприклад інструменти розробника); у повсякденні лишайте вимкненим.' },
}
const devCode = (raw) => {
  const tag = String(raw || 'en').replace(/-/g, '_')
  if (/^zh/i.test(tag) && /(_TW|_HK|_MO|Hant)/i.test(tag)) return 'zh_TW'
  if (/^zh/i.test(tag)) return 'zh'
  const primary = tag.split('_')[0]
  return DEV[tag] ? tag : (DEV[primary] ? primary : 'en')
}
const D = computed(() => DEV[devCode(props.locale)] || DEV.en)
const settings = desktopSettings
const set = (value) => saveDesktopSettings({ ...settings.value, devMode: value }).catch(() => {})
defineExpose({ off: () => set(false) })
</script>
