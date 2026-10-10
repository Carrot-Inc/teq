// node integrations/vite/test.mjs: the plugin's reading of the lock and its search for the
// binary, over a synthetic teq.lock in a scratch directory (at its root, and under its target/teq/
// as without the native driver) and a repository served on the loopback
// interface: TEQ first; the shared cache's copy; sbt-teq's copy, taken only when its sha1 is the
// pinned one; coursier's copy of the URL, copied into the cache; a fetch into the cache, refused
// when the bytes are not the pinned ones. Prints a line per check, FAIL for a failure, and exits 1
// on any.
import { createHash } from "node:crypto"
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs"
import { createServer } from "node:http"
import { tmpdir } from "node:os"
import { join } from "node:path"
import { cacheRoot, classifier, coursierFile, resolveTeq } from "./binary.js"
import { loadDescription } from "./description.js"

let failed = 0
const check = (condition, what, found) => {
  if (condition) console.log(`vite-plugin-teq: ${what}`)
  else {
    console.error(`FAIL vite-plugin-teq: ${what}${found === undefined ? "" : `: found ${JSON.stringify(found)}`}`)
    failed++
  }
}
const throws = async (run) => {
  try {
    await run()
  } catch (err) {
    return err.message
  }
}

const work = mkdtempSync(join(tmpdir(), "vite-plugin-teq-"))
const binary = Buffer.from("#!/bin/sh\necho teq 0.1.0-pre.9\n")
const sha1 = createHash("sha1").update(binary).digest("hex")
const platform = classifier()
const name = `teq-0.1.0-pre.9-${platform}${platform.startsWith("windows") ? ".exe" : ""}`
const served = []
const server = createServer((req, res) => {
  served.push(req.url)
  if (req.url.startsWith("/missing/")) return res.writeHead(404).end()
  res.writeHead(200).end(binary)
})
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve))
const repository = `http://127.0.0.1:${server.address().port}`

/** A tree as a lock: mappings as blocks, a list's mappings on one line, every string quoted. */
function lockText(tree) {
  const scalar = (v) => (typeof v === "string" ? JSON.stringify(v) : String(v))
  const flow = (v) => (v !== null && typeof v === "object" && !Array.isArray(v) ? `{${Object.entries(v).map(([k, x]) => `${JSON.stringify(k)}: ${flow(x)}`).join(", ")}}` : scalar(v))
  const lines = []
  const block = (v, depth) => {
    for (const [k, x] of Object.entries(v)) {
      const head = `${"  ".repeat(depth)}${k === "teq" || k === "format" ? k : JSON.stringify(k)}:`
      if (Array.isArray(x) && x.length) {
        lines.push(head)
        for (const e of x) lines.push(`${"  ".repeat(depth + 1)}- ${flow(e)}`)
      } else if (x !== null && typeof x === "object" && !Array.isArray(x) && Object.keys(x).length) {
        lines.push(head)
        block(x, depth + 1)
      } else lines.push(`${head} ${Array.isArray(x) ? "[]" : typeof x === "object" ? "{}" : scalar(x)}`)
    }
  }
  block(tree, 0)
  return lines.join("\n") + "\n"
}

/** The build's root with its lock: `web` (Scala.js) and `api` (JVM), the binary pinned at `url` + `path`;
 * `place` where the lock lies under the root, `target/teq/teq.lock` without the native driver. */
function build(dir, { path = "public/teq", pin = sha1, url = `${repository}/`, format = 1, place = "teq.lock" } = {}) {
  mkdirSync(join(dir, "web", "app"), { recursive: true })
  mkdirSync(join(dir, place, ".."), { recursive: true })
  const lock = {
    teq: "0.1.0-pre.9",
    format,
    binaries: { [platform]: `${url}${path} ${pin} ${binary.length}` },
    projects: {
      web: {
        base: "web",
        platform: "js",
        configurations: { compile: { sources: ["web/src"], classpath: [{ project: "shared", configuration: "compile" }] } },
        description: { sources: ["shared/src", "web/src"], modulePerFile: ["app.page"], keys: { hotSwapUpTo: "7" } },
      },
      api: { base: "api", platform: "jvm", configurations: {}, description: {} },
    },
  }
  writeFileSync(join(dir, place), lockText(lock))
  return dir
}

const env = { ...process.env }
try {
  delete process.env.TEQ
  process.env.TEQ_CACHE_DIR = join(work, "cache")
  process.env.COURSIER_CACHE = join(work, "coursier")

  const root = build(join(work, "b"))
  const web = loadDescription("web", join(root, "web", "app"))
  check(web.root === root && web.inputs.join() === "shared/src,web/src" && web.modulePerFile.join() === "app.page" && web.hotSwapUpTo === 7, "the lock at or above vite's root: the project's sources, per-file packages and hotSwapUpTo", web)
  const under = build(join(work, "t"), { place: "target/teq/teq.lock" })
  const target = loadDescription("web", join(under, "web", "app"))
  check(target.file === join(under, "target", "teq", "teq.lock") && target.root === under && target.inputs.join() === "shared/src,web/src", "the lock under the root's target/teq/, found from below the root: the build's root is the root", target)
  writeFileSync(join(under, "teq.lock"), readFileSync(join(under, "target", "teq", "teq.lock")))
  check(loadDescription("web", join(under, "web")).file === join(under, "teq.lock"), "a lock at the root comes before the one under its target/teq/")
  check(/is a JVM project/.test(await throws(() => loadDescription("api", root))), "a JVM project is refused")
  check(/has no project nope \(it has web, api\)/.test(await throws(() => loadDescription("nope", root))), "an unknown project is refused, naming the projects")
  check(/no teq.lock at or above .*: export the build with sbt teqExportAll/.test(await throws(() => loadDescription("web", tmpdir()))), "no lock at or above vite's root is refused")
  check(
    /format 2, which this vite-plugin-teq does not read \(it reads format 1\): update vite-plugin-teq to the release of teq 0.1.0-pre.9/.test(await throws(() => loadDescription("web", build(join(work, "f2"), { format: 2 })))),
    "a lock of another format is refused, naming the teq it pins",
  )
  writeFileSync(join(work, "f2", "teq.lock"), 'teq: 0.1.0\nformat: 1\nbinaries: {}\nprojects: {a: 1, a: 2}\n')
  check(/teq.lock: line 4: Map keys must be unique/.test(await throws(() => loadDescription("web", join(work, "f2")))), "a lock YAML refuses is refused with its line")

  process.env.TEQ = "/opt/teq"
  check((await resolveTeq(web)) === "/opt/teq", "TEQ comes first")
  delete process.env.TEQ

  const copy = join(root, "web", "target", "teq", "bin", name)
  mkdirSync(join(copy, ".."), { recursive: true })
  writeFileSync(copy, binary)
  check((await resolveTeq(web)) === copy && served.length === 0, "sbt-teq's copy under the project's target/teq/bin, its sha1 the pinned one")
  writeFileSync(copy, Buffer.from(binary).fill(0x20, 0, 2))
  const cached = join(cacheRoot(), "bin", sha1, name)
  const fetched = await resolveTeq(web)
  check(fetched === cached && served.join() === "/public/teq", "a copy of another sha1 is passed over, and the pinned binary fetched", [fetched, served])
  check(statSync(cached).size === binary.length && (process.platform === "win32" || (statSync(cached).mode & 0o111) !== 0), "into the shared cache, bin/<sha1>/teq-<version>-<classifier>, executable")
  check(readdirSync(join(cached, "..")).join() === name, "with no partial file left beside it", readdirSync(join(cached, "..")))
  served.length = 0
  check((await resolveTeq(web)) === cached && served.length === 0, "the cache's copy is taken without a request")

  const wrong = loadDescription("web", build(join(work, "c"), { pin: "0".repeat(40) }))
  const refused = await throws(() => resolveTeq(wrong))
  check(/has sha1 [0-9a-f]{40} and \d+ bytes where the lock pins 0{40}/.test(refused ?? "") && readdirSync(join(cacheRoot(), "bin", "0".repeat(40))).length === 0, "a fetched binary that is not the pinned one is refused, nothing left in the cache", refused)

  rmSync(join(cacheRoot(), "bin"), { recursive: true, force: true })
  const missing = await throws(() => resolveTeq(loadDescription("web", build(join(work, "m"), { path: "missing/teq" }))))
  check(/GET http:\/\/127\.0\.0\.1:\d+\/missing\/teq answered 404/.test(missing ?? ""), "a binary the repository does not have: the answer", missing)

  rmSync(join(cacheRoot(), "bin"), { recursive: true, force: true })
  const fromCoursier = loadDescription("web", build(join(work, "k"), { path: "coursier/teq" }))
  const kept = coursierFile(process.env.COURSIER_CACHE, `${repository}/coursier/teq`)
  mkdirSync(join(kept, ".."), { recursive: true })
  writeFileSync(kept, binary)
  served.length = 0
  check((await resolveTeq(fromCoursier)) === join(cacheRoot(), "bin", sha1, name) && served.length === 0, "coursier's copy of the URL, its sha1 the pinned one, is copied into the cache without a request", served)
  check(kept === join(process.env.COURSIER_CACHE, "http", `127.0.0.1%3A${server.address().port}`, "coursier", "teq"), "coursier's copy is at its escaped place", kept)

  rmSync(join(cacheRoot(), "bin"), { recursive: true, force: true })
  const hosts = ["http://127.repo.invalid/", "http://127.0.0.1@repo.invalid/", "http://localhost.repo.invalid/"]
  const refusedHosts = []
  for (const [i, url] of hosts.entries()) {
    const message = await throws(() => resolveTeq(loadDescription("web", build(join(work, `h${i}`), { url }))))
    if (/is not https, which teq fetches over from every host but this one/.test(message ?? "")) refusedHosts.push(url)
  }
  check(refusedHosts.length === hosts.length, "plain http to a host that only looks like the loopback is refused", refusedHosts)

  const cwd = process.cwd()
  process.chdir(work)
  process.env.TEQ_CACHE_DIR = "relative-cache"
  const relativeCache = cacheRoot()
  process.env.TEQ = "bin/teq"
  const relativeTeq = await resolveTeq(web)
  process.env.TEQ = "teq"
  const bareTeq = await resolveTeq(web)
  delete process.env.TEQ
  process.chdir(cwd)
  const real = realpathSync(work)
  check(relativeCache === join(real, "relative-cache") && relativeTeq === join(real, "bin/teq") && bareTeq === "teq", "a relative cache or TEQ is the working directory's, since teq runs from the build's root; a bare TEQ is the PATH's", [relativeCache, relativeTeq, bareTeq])

  check(classifier("darwin", "arm64") === "osx-aarch_64" && classifier("linux", "x64") === "linux-x86_64" && classifier("win32", "x64") === "windows-x86_64", "the platforms by the publisher's classifiers")
} finally {
  for (const key of ["TEQ", "TEQ_CACHE_DIR", "COURSIER_CACHE"]) if (env[key] === undefined) delete process.env[key]; else process.env[key] = env[key]
  server.close()
  rmSync(work, { recursive: true, force: true })
}
console.log(failed ? `vite-plugin-teq: ${failed} failed` : "vite-plugin-teq: all passed")
process.exit(failed ? 1 : 0)
