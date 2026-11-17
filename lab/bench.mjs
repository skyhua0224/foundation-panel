import { readFileSync, mkdirSync, writeFileSync, existsSync } from "node:fs"
import { execFileSync, spawnSync } from "node:child_process"
import { join, resolve } from "node:path"
const cfg = JSON.parse(readFileSync("lab/bench-inputs.json", "utf8"))
mkdirSync("bench-result", { recursive: true })
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const within = (p, ms, what) => Promise.race([p, new Promise((_, rej) => setTimeout(() => rej(new Error(what + " timed out")), ms))])
function exeOf(name) {
  for (const p of [`bench/${name}/app/sunshine-gui.exe`, `bench/${name}/sunshine-gui.exe`]) if (existsSync(p)) return resolve(p)
  throw new Error("no exe for " + name)
}
function kill(pid) { spawnSync("taskkill", ["/F", "/T", "/PID", String(pid)]); spawnSync("taskkill", ["/F", "/IM", "msedgewebview2.exe"]) }
async function cdp(port, t0) {
  const out = []
  for (let i = 0; i < 60; i++) {
    let targets
    try { targets = await (await fetch(`http://127.0.0.1:${port}/json`)).json() } catch { await sleep(100); continue }
    const pages = targets.filter((t) => t.type === "page")
    if (!pages.length) { await sleep(100); continue }
    for (const t of pages) {
      let ws, v
      try {
      ws = new WebSocket(t.webSocketDebuggerUrl)
      await within(new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej }), 3000, "ws open")
      v = await within(new Promise((res) => {
        ws.onmessage = (m) => { const d = JSON.parse(m.data); if (d.id === 1) res(d.result?.result?.value) }
        ws.send(JSON.stringify({ id: 1, method: "Runtime.evaluate", params: { returnByValue: true, expression:
          `JSON.stringify({o: performance.timeOrigin, fcp: (performance.getEntriesByName("first-contentful-paint")[0]||{}).startTime ?? null, lcp: null, dcl: performance.getEntriesByType("navigation")[0]?.domContentLoadedEventEnd ?? null, load: performance.getEntriesByType("navigation")[0]?.loadEventEnd ?? null, url: location.href})` } }))
      }), 3000, "evaluate")
      } catch (e) { console.log("cdp", e.message) }
      try { ws?.close() } catch {}
      if (v) { const j = JSON.parse(v); out.push({ url: j.url, fcpSinceStart: j.fcp == null ? null : j.o + j.fcp - t0, dclSinceStart: j.dcl == null ? null : j.o + j.dcl - t0, loadSinceStart: j.load == null ? null : j.o + j.load - t0 }) }
    }
    if (out.some((x) => x.fcpSinceStart != null)) return out
    out.length = 0
    await sleep(200)
  }
  return out
}
const results = Object.fromEntries(cfg.variants.map((v) => [v.name, []]))
const profiles = Object.fromEntries(cfg.variants.map((v) => [v.name, resolve(`bench/wv2-${v.name}`)]))
let port = 9500
async function trial(v, idx, warm) {
  port++
  const r = spawnSync("pwsh", ["-NoProfile", "-File", "lab/measure.ps1", "-Exe", exeOf(v.name), "-Port", String(port), "-Profile", profiles[v.name]], { encoding: "utf8", timeout: 90000 })
  let m
  try { m = JSON.parse(r.stdout.trim().split("\n").pop()) } catch { console.log("measure failed", r.stdout, r.stderr); return }
  await sleep(1500)
  const pages = await cdp(port, m.t0)
  kill(m.pid)
  await sleep(1500)
  if (warm) return
  const row = { windowMs: m.windowMs, exited: m.exited, pages }
  results[v.name].push(row)
  console.log(v.name, idx, JSON.stringify(row))
}
for (const v of cfg.variants) await trial(v, -1, true)
for (let i = 0; i < cfg.trials; i++) {
  const order = i % 2 ? [...cfg.variants].reverse() : cfg.variants
  for (const v of order) await trial(v, i, false)
}
writeFileSync("bench-result/raw.json", JSON.stringify(results, null, 2))
const pct = (a, p) => { const s = [...a].sort((x, y) => x - y); return s[Math.min(s.length - 1, Math.floor(p * s.length))] }
const summ = {}
for (const [name, rows] of Object.entries(results)) {
  const win = rows.map((r) => r.windowMs).filter((x) => x > 0)
  const fcp = rows.map((r) => Math.min(...r.pages.map((p) => p.fcpSinceStart ?? Infinity))).filter(Number.isFinite)
  summ[name] = { n: rows.length, window: { p50: pct(win, 0.5), p95: pct(win, 0.95), mean: win.reduce((a, b) => a + b, 0) / (win.length || 1) }, fcp: { n: fcp.length, p50: pct(fcp, 0.5), p95: pct(fcp, 0.95), mean: fcp.reduce((a, b) => a + b, 0) / (fcp.length || 1) } }
}
console.log(JSON.stringify(summ, null, 2))
writeFileSync("bench-result/summary.json", JSON.stringify(summ, null, 2))
