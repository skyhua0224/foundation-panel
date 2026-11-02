import { parsePanelLogs, parseRecords } from 'foundation-ui/diagnostics'

/**
 * The host log and the panel's own entries as one record list. source.id keeps
 * them apart for the viewer filter; the order is host first, then panel.
 */
export function combineDiagnosticsRecords(hostText, panelEntries) {
  return [...parseRecords(hostText), ...parsePanelLogs(panelEntries)]
}
