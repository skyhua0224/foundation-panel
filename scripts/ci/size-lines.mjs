// Parses the `llvm-pdbutil dump -l` text: which source file each address range of one
// section belongs to. Source: panel-llvm-pdbutil (dump -l; DumpOutputStyle prints one
// block per file, headed by the file name and its checksum, then the ranges as
// `line/addr` or, with column information, `line/column/addr`).

const UNKNOWN = '[other source]'

/**
 * @param {string} dump text of `llvm-pdbutil dump -l`
 * @param {number} sectionIndex 1-based section number, as printed before the colon in a range
 * @param {(path: string) => string} ownerOf maps a path to the name a range is charged to (UNKNOWN when it says nothing)
 * @returns {{lo: number, hi: number, owner: string}[]}
 */
export function lineRanges(dump, sectionIndex, ownerOf) {
  const ranges = []
  let owner = UNKNOWN
  let moduleOwner = UNKNOWN
  for (const line of dump.split('\n')) {
    // A module heading starts a new object file. It names the object, which says whose code a file without a
    // registry path (an assembly file) belongs to; nothing before the first file heading is charged to the file before.
    if (line.startsWith('Mod ')) {
      const object = line.match(/`(.*)`/)
      moduleOwner = (object && ownerOf(object[1])) || UNKNOWN
      owner = moduleOwner
      continue
    }
    // A file heading is not indented and ends with its checksum: `<path> (MD5: ...)`, `(SHA-256: ...)` or `(None)`.
    const heading = line.match(/^(\S.*?) \((?:MD5|SHA-1|SHA-256|None)\b[^)]*\)\s*$/)
    if (heading) {
      const named = ownerOf(heading[1])
      owner = named && named !== UNKNOWN ? named : moduleOwner
      continue
    }
    const range = line.match(/^\s+(\d+):([0-9A-Fa-f]+)-([0-9A-Fa-f]+), line\/(?:column\/)?addr/)
    if (!range || Number(range[1]) !== sectionIndex) continue
    ranges.push({ lo: parseInt(range[2], 16), hi: parseInt(range[3], 16), owner })
  }
  return ranges
}
