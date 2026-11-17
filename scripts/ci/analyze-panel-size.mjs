// Sources: panel-ms-pe-format (Section Table), panel-llvm-pdbutil (dump),
// panel-tauri-codegen-assets (compress_file), panel-ms-x64-unwind
// (RUNTIME_FUNCTION). No third-party packages needed.
// Tool APIs: panel-node-zlib (BrotliOptions/bytesWritten), panel-node-fs
// (readFileSync), panel-node-child-process (execFileSync), panel-node-crypto
// (createHash/digest).
import { readFileSync } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { createHash } from 'node:crypto'
import { brotliDecompressSync } from 'node:zlib'

const [exePath, pdbPath, pdbutil = 'llvm-pdbutil'] = process.argv.slice(2)
if (!exePath || !pdbPath) {
  throw new Error('Usage: node analyze-panel-size.mjs <exe> <matching-pdb> [llvm-pdbutil]')
}
const exe = readFileSync(exePath)
const hash = (data) => createHash('sha256').update(data).digest('hex')
const pe = exe.readUInt32LE(60)
if (exe.toString('ascii', pe, pe + 4) !== 'PE\0\0') throw new Error('Expected a PE image')
if (exe.readUInt16LE(pe + 4) !== 0x8664) throw new Error('Expected an x64 PE image')
const optional = pe + 24
const table = optional + exe.readUInt16LE(pe + 20)
const count = exe.readUInt16LE(pe + 6)
const sections = []
for (let i = 0; i < count; ++i) {
  const entry = table + i * 40
  const section = {
    index: i + 1,
    name: exe.toString('ascii', entry, entry + 8).replace(/\0.*$/, ''),
    virtual_bytes: exe.readUInt32LE(entry + 8),
    rva: exe.readUInt32LE(entry + 12),
    raw_bytes: exe.readUInt32LE(entry + 16),
    offset: exe.readUInt32LE(entry + 20),
  }
  if (section.offset + section.raw_bytes > exe.length) throw new Error('Truncated PE section')
  sections.push(section)
}
const rdata = sections.find((s) => s.name === '.rdata')
const text = sections.find((s) => s.name === '.text')
if (!rdata || !text) throw new Error('Expected .text and .rdata')
const dump = (...flags) => execFileSync(pdbutil, ['dump', ...flags, pdbPath], {
  encoding: 'utf8', maxBuffer: 512 * 1024 * 1024,
})
const layout = dump('-summary', '-section-contribs')

// Require the CodeView GUID and age to match before using PDB offsets.
const guid = layout.match(/GUID: \{([A-Fa-f0-9-]+)\}/)?.[1]?.replaceAll('-', '').toLowerCase()
const age = Number(layout.match(/Age: (\d+)/)?.[1])
const guidBytes = guid && Buffer.from(guid, 'hex')
if (!guidBytes) throw new Error('PDB has no GUID')
const codeviewGuid = Buffer.from(guidBytes)
codeviewGuid.writeUInt32LE(guidBytes.readUInt32BE(0), 0)
codeviewGuid.writeUInt16LE(guidBytes.readUInt16BE(4), 4)
codeviewGuid.writeUInt16LE(guidBytes.readUInt16BE(6), 6)
const rsds = Buffer.concat([Buffer.from('RSDS'), codeviewGuid, Buffer.alloc(4)])
rsds.writeUInt32LE(age, 20)
if (!exe.includes(rsds)) throw new Error('PDB GUID/age does not match the executable')

function classify(bytes) {
  if (bytes.subarray(0, 4).toString() === 'wOF2') return 'font-woff2'
  if (bytes.subarray(0, 4).toString() === 'wOFF') return 'font-woff'
  if (bytes.subarray(0, 8).equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]))) return 'image-png'
  if (bytes.subarray(0, 4).toString() === 'RIFF' && bytes.subarray(8, 12).toString() === 'WEBP') return 'image-webp'
  if (bytes.subarray(0, 3).equals(Buffer.from([255, 216, 255]))) return 'image-jpeg'
  if (bytes.subarray(0, 4).equals(Buffer.from([0, 0, 1, 0]))) return 'image-ico'
  const prefix = bytes.subarray(0, 256).toString('utf8').trimStart()
  if (/^(<!doctype|<html)/i.test(prefix)) return 'html'
  if (/^(<svg|<\?xml)/.test(prefix)) return 'svg-or-xml'
  if (/^(@font-face|@charset|:root|\/\*)/.test(prefix)) return 'css-or-text'
  return 'script-or-other'
}

const assets = []
const contributionMap = new Map()
for (const m of layout.matchAll(/SC\[\.rdata\]\s+\| mod = \d+, (\d+):(\d+), size = (\d+)/g)) {
  const [section, offset, size] = m.slice(1).map(Number)
  if (section !== rdata.index || offset + size > rdata.raw_bytes || contributionMap.has(offset)) continue
  contributionMap.set(offset, { offset, size })
  if (size < 32) continue
  const packed = exe.subarray(rdata.offset + offset, rdata.offset + offset + size)
  let decoded
  try { decoded = brotliDecompressSync(packed, { maxOutputLength: 32 * 1024 * 1024, info: true }) } catch { continue }
  // A complete stream consuming the entire contribution avoids counting
  // accidental decodable prefixes of unrelated constants or pointers.
  if (decoded.engine.bytesWritten !== size || decoded.buffer.length < 32) continue
  assets.push({ offset, packed_bytes: size, raw_bytes: decoded.buffer.length,
    category: classify(decoded.buffer), raw_sha256: hash(decoded.buffer) })
}

// Locate contributions reached by x64 runtime-function unwind addresses.
// Their whole sizes can include pooled/language-specific records; do not call
// this an exact byte count of individual UNWIND_INFO structures.
const contributions = [...contributionMap.values()].sort((a, b) => a.offset - b.offset)
const unwindContributions = new Set(), unwindAddresses = new Set()
const pdata = sections.find((s) => s.name === '.pdata')
if (!pdata || pdata.virtual_bytes % 12) throw new Error('Expected an x64 runtime-function table')
for (let pos = pdata.offset; pos < pdata.offset + pdata.virtual_bytes; pos += 12) {
  const rva = exe.readUInt32LE(pos + 8)
  unwindAddresses.add(rva)
  const offset = rva - rdata.rva
  let lo = 0, hi = contributions.length
  while (lo < hi) {
    const mid = (lo + hi) >>> 1
    if (contributions[mid].offset <= offset) lo = mid + 1
    else hi = mid
  }
  const c = contributions[lo - 1]
  if (c && offset < c.offset + c.size) unwindContributions.add(c.offset)
}
if (assets.some((a) => unwindContributions.has(a.offset))) throw new Error('Asset/unwind attribution overlap')
const unwindContributionBytes = contributions.filter((c) => unwindContributions.has(c.offset))
  .reduce((sum, c) => sum + c.size, 0)

function crateOf(path) {
  const normal = path.replaceAll('\\', '/')
  const registry = normal.match(/\/registry\/src\/[^/]+\/([\w-]+)-(\d[^/]*)\//)
  if (registry) return `${registry[1]} ${registry[2]}`
  const std = normal.match(/\/library\/([^/]+)\//)
  if (std) return std[1]
  if (normal.includes('/src-tauri/src/') || normal.startsWith('src/')) return 'sunshine-gui'
  return '[other source]'
}

// Attribute disjoint line-table address ranges; overlapping folded ranges
// from different crates are reported separately, never counted twice.
const events = []
let owner = '[other source]'
for (const line of dump('-l').split('\n')) {
  if (/\.(rs|c|cpp|cc|h) \(.*\)/.test(line)) owner = crateOf(line)
  const range = line.match(/^\s+(\d+):([0-9A-Fa-f]+)-([0-9A-Fa-f]+), line\/addr/)
  if (!range || Number(range[1]) !== text.index) continue
  const lo = parseInt(range[2], 16), hi = parseInt(range[3], 16)
  if (lo >= hi || hi > text.virtual_bytes) throw new Error('PDB line range outside .text')
  events.push([lo, owner, 1], [hi, owner, -1])
}
events.sort((a, b) => a[0] - b[0])
const active = new Map(), totals = new Map()
let previous = 0
for (const [position, name, delta] of events) {
  const owners = [...active.keys()]
  const label = owners.length === 0 ? '[no line table]' : owners.length === 1 ? owners[0] : '[folded/mixed]'
  totals.set(label, (totals.get(label) || 0) + position - previous)
  const next = (active.get(name) || 0) + delta
  if (next) active.set(name, next)
  else active.delete(name)
  previous = position
}
totals.set('[no line table]', (totals.get('[no line table]') || 0) + text.virtual_bytes - previous)
const byCategory = {}
for (const a of assets) {
  const row = byCategory[a.category] ??= { count: 0, packed_bytes: 0, raw_bytes: 0 }
  ++row.count; row.packed_bytes += a.packed_bytes; row.raw_bytes += a.raw_bytes
}
const attributed = [...totals].filter(([k]) => !k.startsWith('[')).reduce((sum, [, n]) => sum + n, 0)
console.log(JSON.stringify({
  schema: 1,
  exe_bytes: exe.length,
  exe_sha256: hash(exe),
  pdb_sha256: hash(readFileSync(pdbPath)),
  pdb_guid_age_match: true,
  sections,
  attribution_method: 'Non-overlapping PDB source-line ranges after LTO; folded/mixed and missing ranges kept separate. This is source attribution, not pre-LTO crate object sizes.',
  text_attributed_fraction: attributed / text.virtual_bytes,
  text_by_crate: [...totals].sort((a, b) => b[1] - a[1]).map(([crate, bytes]) => ({ crate, bytes })),
  rdata_brotli_by_category: byCategory,
  rdata_brotli_bytes: assets.reduce((sum, a) => sum + a.packed_bytes, 0),
  rdata_other_bytes: rdata.raw_bytes - assets.reduce((sum, a) => sum + a.packed_bytes, 0),
  runtime_function_count: pdata.virtual_bytes / 12,
  unique_unwind_addresses: unwindAddresses.size,
  rdata_unwind_referenced_contribution_count: unwindContributions.size,
  rdata_unwind_referenced_contribution_bytes: unwindContributionBytes,
  // Unknown bytes include strings, tables, pointers and unwind metadata;
  // this diagnostic does not infer that they are fonts or ICU payloads.
  rdata_brotli_streams: assets.sort((a, b) => b.packed_bytes - a.packed_bytes),
}, null, 2))
