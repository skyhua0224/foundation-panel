import { dlssNr } from '../tauri-adapter.js'
import { createDlssNrReady } from './dlssNrReady.js'

// The one shared DLSS NR status of the panel (see dlssNrReady.js).
const store = createDlssNrReady(() => dlssNr.getStatus())
/** `shown`: true only when DLSS NR is known to be installed and usable. */
export const useDlssNrReady = () => store.use()
/** The same answer without reading: for a page that reads the status itself and hands it in (画质增强). */
export const dlssNrShown = store.shown
/** Hand in a DLSS NR component status read elsewhere (EnhancementManager, 画质增强). */
export const noteDlssNrStatus = (status) => store.note(status)
