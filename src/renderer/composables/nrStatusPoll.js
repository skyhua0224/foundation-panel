// A save during a status read invalidates that read and requests one fresh snapshot.
export function createNrStatusPoll(read) {
  let revision = 0
  let running = false

  async function poll() {
    if (running) return
    running = true
    try {
      while (true) {
        const started = revision
        await read(() => started === revision)
        if (started === revision) break
      }
    } finally {
      running = false
    }
  }

  function refresh() {
    ++revision
    return poll()
  }
  return { poll, refresh }
}
