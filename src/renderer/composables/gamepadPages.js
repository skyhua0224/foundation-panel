import { pageBack } from 'foundation-ui/settings/places.js'
import { ROUTES } from './useRouter.js'

// The panel's gamepad page is one page with two ways in: 输入与外设 > 手柄 (the rail destination) and 输入 > 手柄 > DualSense of the
// stream settings (the page the settings rows and the search open). DualSense is never a destination of its own.
export const GAMEPAD_PAGE_ROUTES = Object.freeze([ROUTES.GAMEPAD, ROUTES.CFG_DUALSENSE])

/** True when the gamepad page was opened as the DualSense page of the stream settings (it then has a back bubble at its root). */
export const isDualSenseEntry = (route) => route === ROUTES.CFG_DUALSENSE

/** The settings destination the back bubble of the DualSense page returns to: the 输入 tab of the stream settings. */
export function dualsenseBack() {
  const target = pageBack('dualsense')
  return { route: ROUTES.WEB_CONFIG, params: target.params }
}
