import { saveThroughDialog } from '../shared/saveDialog.js'

/** Export the panel log through its save dialog. Cancel is a no-op; a failure is shown with its reason. */
export async function exportPanelLogs({ invoke, format, texts, alert }) {
  try {
    const saved = await saveThroughDialog(invoke, 'export_logs', { format })
    if (saved === null) return false
    await alert(saved || texts.exportSuccess, texts.exportComplete, { type: 'info' })
    return true
  } catch (error) {
    await alert(`${texts.exportFailed}: ${error}`, texts.dialogError, { type: 'error' })
    return false
  }
}
