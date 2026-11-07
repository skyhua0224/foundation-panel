// Back of a framed Web page. Where the list is hidden behind the page (single pane) the page itself draws the arrow in its title row, the way the panel's own pages do; the panel only says when Back
// applies and takes the press. A Web that does not say that it drew the arrow (an older build, a page without a title row) keeps the panel's own bar, so no pair of builds is left without a Back.

export const PANEL_BACK = 'foundation-panel-back'
export const BACK_REPORT = 'foundation-back-inline'
export const BACK_CLICK = 'foundation-back-click'
export const BACK_REQUEST = 'foundation-back-request'

/** what the panel tells the frame: whether Back applies (the scene has a list behind the page) */
export const panelBackMessage = (show) => ({ type: PANEL_BACK, show: show === true })

/** What a frame said about Back, or null: `inline` (a page of it draws the arrow, or does not), `click` (the arrow was pressed), `request` (a new document asks for the state). */
export function readFrameBack(data) {
  if (!data || typeof data !== 'object') return null
  if (data.type === BACK_REPORT && typeof data.drawn === 'boolean') return { kind: 'inline', drawn: data.drawn }
  if (data.type === BACK_CLICK) return { kind: 'click' }
  if (data.type === BACK_REQUEST) return { kind: 'request' }
  return null
}

/** The panel's own Back bar: only where Back applies, the framed page has no back bubble of its own (a sub-page) and the page did not draw the arrow itself. */
export const ownBackBar = ({ canBack, sub, inline }) => canBack === true && !sub && !inline
