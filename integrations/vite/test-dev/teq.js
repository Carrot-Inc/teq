import { mkdirSync, writeFileSync } from "node:fs"
import { join } from "node:path"

export async function build() {
  throw new Error("test-dev: no production build")
}
export function isGiveWayNote() {
  return false
}

const ok = (changed, modules) => ({ ok: true, incremental: true, changed, modules, ms: { type: 0, reach: 0, emit: 0, write: 0, total: 0 }, warnings: [] })

/** Writes the modules of `globalThis.teqUnderTest` (set by the test) instead of running teq. */
export class TeqWatch {
  constructor(runner, args) {
    this.out = args[args.indexOf("--split") + 1]
  }
  write(name, text) {
    const file = join(this.out, name)
    mkdirSync(join(file, ".."), { recursive: true })
    writeFileSync(file, text)
  }
  async start() {
    const modules = globalThis.teqUnderTest.first
    for (const [name, text] of Object.entries(modules)) this.write(name, text)
    return ok(Object.keys(modules), Object.keys(modules).length)
  }
  async build() {
    const changed = globalThis.teqUnderTest.next
    for (const [name, text] of Object.entries(changed)) this.write(name, text)
    return ok(Object.keys(changed), Object.keys(globalThis.teqUnderTest.first).length)
  }
  stop() {}
}
