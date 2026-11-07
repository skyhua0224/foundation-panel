import assert from 'node:assert/strict'
import fs from 'node:fs'
import test from 'node:test'

// A route of a Web page never shows another page's content: the shell tells the frame whenever the route's page is not the one it shows (a deep link, a caller that only switches the route).
const shell = fs.readFileSync(new URL('../components/SidebarMenu.vue', import.meta.url), 'utf8')
test('the frame follows every route that has a Web path', () => {
  const at = shell.indexOf('watch(() => router.currentRoute.value, (route) => {\n  const path = webPathOf(route)')
  assert.ok(at > 0, 'a watch on the current route that reads the Web path of it')
  const body = shell.slice(at, shell.indexOf('immediate: true })', at) + 'immediate: true })'.length)
  assert.match(body, /emit\('webui-navigate', path\)/)
  assert.match(body, /routeForWebPath\(props\.webuiPath\) !== route/, 'not when the frame already shows the page of the route')
  assert.match(body, /flush: 'post'/, 'after the frame path of a caller that told the frame itself has arrived')
})

// the back stack mirrors into the window's history, or the back arrow of a pushed page (关于, 公告) goes back in the history and the page stays
test('the shell attaches the back stack to the window history', () => {
  assert.match(shell, /onMounted\(\(\) => \{ nav\.widen\(\); nav\.attach\(\) \}\)/)
  assert.match(shell, /onBeforeUnmount\(\(\) => nav\.detach\(\)\)/)
})
