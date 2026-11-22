// The update state of the tile 软件更新 on 帮助与关于 (foundation-ui MHelpTiles), from what the panel already knows. Nothing here asks the network.
//   checking: the person's own check on the tile is running
//   waiting:  a version waits and the person did not skip it (the notice of foundation-ui/update)
//   failed:   the person's own check on the tile found nothing out (the checker answered with nothing)
//   answered: a check has answered since the panel started (a result, up to date or not)
export const helpUpdateState = ({ checking = false, waiting = false, failed = false, answered = false } = {}) => (checking ? 'checking' : waiting ? 'available' : failed ? 'error' : answered ? 'latest' : 'idle')

/** What the tile gets: the state, the installed version, the version that waits and when the last check ended. */
export const helpUpdateView = (flags, { installed = '', next = '', checkedAt = 0 } = {}) => ({ state: helpUpdateState(flags), installed, next, checkedAt })
