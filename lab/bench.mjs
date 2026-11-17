import { readFileSync, mkdirSync, writeFileSync, existsSync } from "node:fs"
import { spawnSync } from "node:child_process"
import { resolve } from "node:path"
const cfg = JSON.parse(readFileSync("lab/bench-inputs.json", "utf8"))
mkdirSync("bench-result", { recursive: true })
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const exeOf = (name) => { for (const p of [`bench/${name}/app/sunshine-gui.exe`, `bench/${name}/sunshine-gui.exe`]) if (existsSync(p)) return resolve(p); throw new Error("no exe for " + name) }
const cleanup = () => { spawnSync("taskkill", ["/F", "/T", "/IM", "sunshine-gui.exe"]); spawnSync("taskkill", ["/F", "/IM", "msedgewebview2.exe"]) }
const results = Object.fromEntries(cfg.variants.map((v) => [v.name, []]))
const profiles = Object.fromEntries(cfg.variants.map((v) => [v.name, resolve(`bench/wv2-${v.name}`)]))
async function trial(v, idx, warm) {
  const r = spawnSync("pwsh", ["-NoProfile", "-File", "lab/measure.ps1", "-Exe", exeOf(v.name), "-Port", "0", "-Profile", profiles[v.name], "-ObserveMs", String(cfg.observeMs || 7000)], { encoding: "utf8", timeout: 90000 })
  let row
  try {
    const m = JSON.parse(r.stdout.trim().split("\n").pop())
    const [head, series] = m.result.split("|")
    const [visible, first, settled, content, frames, pid] = head.split(",").map(Number)
    row = { visible, firstChange: first, content, frames }
  } catch (e) { console.log("measure failed", v.name, idx, r.stdout?.slice(0, 600), r.stderr?.slice(0, 600)); cleanup(); await sleep(2000); return }
  cleanup()
  await sleep(1500)
  if (warm) return
  results[v.name].push(row)
  console.log(v.name, idx, JSON.stringify(row).slice(0, 700))
}
for (const v of cfg.variants) await trial(v, -1, true)
for (let i = 0; i < cfg.trials; i++) {
  const order = i % 2 ? [...cfg.variants].reverse() : cfg.variants
  for (const v of order) await trial(v, i, false)
}
writeFileSync("bench-result/raw.json", JSON.stringify(results, null, 2))
const pct = (a, p) => { const s = [...a].sort((x, y) => x - y); return s.length ? s[Math.min(s.length - 1, Math.floor(p * s.length))] : null }
const stat = (a) => ({ n: a.length, p50: pct(a, 0.5), p95: pct(a, 0.95), mean: a.length ? a.reduce((x, y) => x + y, 0) / a.length : null })
const summ = {}
for (const [name, rows] of Object.entries(results)) {
  const ok = (k) => rows.map((r) => r[k]).filter((x) => x > 0)
  summ[name] = { visible: stat(ok("visible")), firstChange: stat(ok("firstChange")), content: stat(ok("content")) }
}
console.log(JSON.stringify(summ, null, 2))
writeFileSync("bench-result/summary.json", JSON.stringify(summ, null, 2))
