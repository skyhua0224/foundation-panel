import { ROUTES } from './useRouter.js'
import { WEBUI_PAGES, getWebuiPage } from './webuiNavigation.js'

const WEBUI_ICONS = {
  overview: 'home',
  downloads: 'download',
  apps: 'grid_view',
  pairing: 'link',
  'stream-settings': 'tune',
}

/** Navigation represents destinations; actions and feature status live on their pages. */
export function createNavigationGroups(ctx) {
  const labels = ctx.t.value.sidebar
  const routeItem = (id, icon, label, route) => ({
    id,
    icon,
    label,
    action: () => ctx.router.navigate(route),
    isActive: () => ctx.router.isRoute(route),
  })
  return [
    {
      id: 'stream',
      label: labels.sectionStream,
      items: WEBUI_PAGES.map((page) => ({
        id: page.id,
        icon: WEBUI_ICONS[page.id],
        label: labels[page.label],
        action: () => ctx.openWebUi(page.path),
        isActive: () => ctx.router.isRoute(ROUTES.HOME) && getWebuiPage(ctx.webuiPath.value)?.id === page.id,
      })),
    },
    {
      id: 'devices',
      label: labels.sectionDevices,
      items: [
        routeItem('vdd', 'monitor', labels.virtualDisplay, ROUTES.VDD_SETTINGS),
        {
          id: 'device-hub',
          icon: 'sports_esports',
          label: labels.deviceHub,
          action: ctx.openControllers,
          isActive: () => ctx.router.isRoute(ROUTES.CONTROLLERS),
        },
      ],
    },
    {
      id: 'assist',
      label: labels.sectionAssist,
      items: [
        routeItem('toolbox', 'build', labels.toolbox, ROUTES.TOOLBOX),
        routeItem('ai-assistant', 'wand_stars', labels.aiAssistant, ROUTES.AI_ASSISTANT),
        ...(import.meta.env.DEV ? [routeItem('web-stream', 'link', labels.webStream, ROUTES.WEB_STREAM)] : []),
      ],
    },
  ]
}

export function createUtilityTools(ctx) {
  const labels = ctx.t.value.sidebar
  return [
    {
      id: 'latency-clock',
      icon: 'timer',
      label: labels.latencyClock,
      description: labels.latencyClockDescription,
      action: () => ctx.router.navigate(ROUTES.LATENCY_CLOCK),
    },
    {
      id: 'test-tools',
      icon: 'science',
      label: labels.testTools,
      description: labels.testToolsDescription,
      action: () => ctx.router.navigate(ROUTES.TEST_TOOLS),
    },
    {
      id: 'stream-timer',
      icon: 'timer',
      label: labels.streamTimer,
      description: labels.timerDescription,
      action: () => ctx.router.navigate(ROUTES.STREAM_TIMER),
    },
    {
      id: 'latency-test',
      icon: 'stacked_line_chart',
      label: labels.latencyTest,
      description: labels.latencyDescription,
      external: true,
      action: () => ctx.openUrl('https://yangkile.github.io/D-lay/'),
    },
    {
      id: 'gamepad-test',
      icon: 'memory',
      label: labels.gamepadTest,
      description: labels.gamepadDescription,
      action: ctx.openGamepadTest,
    },
    {
      id: 'stylus-input-probe',
      icon: 'palette',
      label: ctx.t.value.tools.stylusInputProbe,
      description: labels.stylusDescription,
      action: ctx.openStylusInputProbe,
    },
  ]
}

export function createFooterTools(ctx) {
  return [
    {
      id: 'panel-settings',
      icon: 'settings',
      label: ctx.t.value.sidebar.panelSettings,
      action: () => ctx.router.navigate(ROUTES.PANEL_SETTINGS),
      isActive: () =>
        ctx.router.isRoute(ROUTES.PANEL_SETTINGS) ||
        (ctx.router.isRoute(ROUTES.HOME) && /^\/password(?:\.html)?(?:[?#]|$)/.test(ctx.webuiPath.value)),
    },
    {
      id: 'help',
      icon: 'help',
      label: ctx.t.value.sidebar.helpAbout,
      action: () => ctx.router.navigate(ROUTES.HELP),
      isActive: () =>
        ctx.router.isRoute(ROUTES.HELP) ||
        (ctx.router.isRoute(ROUTES.HOME) && /^\/troubleshooting(?:\.html)?(?:[?#]|$)/.test(ctx.webuiPath.value)),
    },
  ]
}
