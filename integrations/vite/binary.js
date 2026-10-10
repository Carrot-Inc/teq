import { spawn } from "node:child_process"
import { createHash, randomBytes } from "node:crypto"
import { chmodSync, closeSync, copyFileSync, mkdirSync, openSync, readFileSync, renameSync, rmSync, statSync } from "node:fs"
import { isIP } from "node:net"
import { basename, dirname, join, resolve } from "node:path"

/** The publisher's classifier of a platform, as the lock names the binaries: `osx-aarch_64`, `linux-x86_64`, `windows-x86_64`. */
export function classifier(platform = process.platform, arch = process.arch) {
  const os = { darwin: "osx", win32: "windows" }[platform] ?? platform
  const cpu = { arm64: "aarch_64", x64: "x86_64" }[arch] ?? arch
  return `${os}-${cpu}`
}

/**
 * teq's cache, the rule of `src/task/fetch.rs`'s `cache_root`: `TEQ_CACHE_DIR`, else
 * `$XDG_CACHE_HOME/teq`, else `%LOCALAPPDATA%\teq` on Windows, `~/Library/Caches/teq` on macOS
 * and `~/.cache/teq` elsewhere; a relative one is the working directory's, as for teq, since the
 * binary found there runs from the build's root.
 */
export function cacheRoot(env = process.env, platform = process.platform) {
  if (env.TEQ_CACHE_DIR) return resolve(env.TEQ_CACHE_DIR)
  if (env.XDG_CACHE_HOME) return resolve(env.XDG_CACHE_HOME, "teq")
  if (platform === "win32") return env.LOCALAPPDATA ? resolve(env.LOCALAPPDATA, "teq") : undefined
  if (!env.HOME) return undefined
  return resolve(env.HOME, platform === "darwin" ? "Library/Caches/teq" : ".cache/teq")
}

/**
 * Coursier's cache: `COURSIER_CACHE`, else `%LOCALAPPDATA%\\Coursier\\cache\\v1` on Windows,
 * `~/Library/Caches/Coursier/v1` on macOS and `~/.cache/coursier/v1` elsewhere.
 */
export function coursierCache(env = process.env, platform = process.platform) {
  if (env.COURSIER_CACHE) return resolve(env.COURSIER_CACHE)
  if (platform === "win32") return env.LOCALAPPDATA ? resolve(env.LOCALAPPDATA, "Coursier", "cache", "v1") : undefined
  if (!env.HOME) return undefined
  return resolve(env.HOME, platform === "darwin" ? "Library/Caches/Coursier/v1" : ".cache/coursier/v1")
}

/**
 * The file coursier keeps for a URL, as its `CachePath.localFile` names it with no user
 * (`src/task/fetch.rs`'s `coursier_file`): the scheme, a `/`, then the URL after the scheme's `:` with its
 * leading `/`s taken off and a trailing `/` made `/.directory`, the whole escaped as `CachePath.escape`
 * does: every UTF-16 unit above 128 and each of ` %$&+,:;=?@<>#` as `%` and two capital hexadecimal
 * digits (the authority's user, a port, the query and the fragment kept). Undefined for a URL coursier
 * refuses: no scheme, no `/` after it, or a `.` or `..` segment.
 */
export function coursierFile(cache, url) {
  const colon = url.indexOf(":")
  if (!cache || colon < 0) return undefined
  let rest = url.slice(colon + 1)
  if (rest.startsWith("///")) rest = rest.slice(3)
  else if (rest.startsWith("/")) rest = rest.slice(1)
  else return undefined
  if (rest.endsWith("/")) rest += ".directory"
  const place = `${url.slice(0, colon)}/${rest.replace(/^\/+/, "")}`
  const digit = (n) => String.fromCharCode(n < 10 ? 48 + n : 55 + n)
  let escaped = ""
  for (let i = 0; i < place.length; i++) {
    const unit = place.charCodeAt(i)
    escaped += unit > 128 || " %$&+,:;=?@<>#".includes(place[i]) ? `%${digit(Math.floor(unit / 16))}${digit(unit % 16)}` : place[i]
  }
  if (escaped.split("/").some((segment) => segment === "." || segment === "..")) return undefined
  return join(cache, ...escaped.split("/"))
}

/**
 * The teq binary for a project of the export, as every tool finds it (docs/TARGETS.md, "The
 * launchers"): `TEQ`; else the shared cache's `bin/<sha1>/teq-<version>-<classifier>` of the binary
 * the lock's header pins for this platform, which the launchers and sbt-teq fill; else sbt-teq's
 * copy under the project's or the build's `target/teq/bin/`; else coursier's copy of the binary's
 * URL, its sha1 the pinned one, copied into the cache; else fetched into the cache with curl as the
 * launchers fetch it, its size and sha1 the lock's.
 */
export async function resolveTeq(description, log = () => {}) {
  const named = process.env.TEQ
  if (named) return /[\\/]/.test(named) ? resolve(named) : named
  const { version, binaries } = description.pin
  const platform = classifier()
  const fields = Object.hasOwn(binaries, platform) ? binaries[platform] : undefined
  if (!fields) {
    const known = Object.keys(binaries).join(", ") || "none"
    throw new Error(`teq: ${description.file} pins no teq binary for ${platform} (it has: ${known}), and TEQ is not set`)
  }
  const [url, pinnedSha1, sizeText, ...rest] = String(fields).split(" ")
  if (rest.length || !/^[0-9a-fA-F]{40}$/.test(pinnedSha1 ?? "") || !/^[0-9]+$/.test(sizeText ?? "")) {
    throw new Error(`teq: ${description.file}: binaries.${platform} is not \`<url> <sha1> <size>\``)
  }
  const [sha1, size] = [pinnedSha1.toLowerCase(), Number(sizeText)]
  const name = `teq-${version}-${platform}${platform.startsWith("windows") ? ".exe" : ""}`
  const root = cacheRoot()
  const cached = root && join(root, "bin", sha1, name)
  if (cached && sizeOf(cached) === size) return cached
  for (const dir of [description.base, "."]) {
    const copy = join(description.root, dir, "target", "teq", "bin", name)
    if (sizeOf(copy) === size && sha1Of(copy) === sha1) return copy
  }
  if (!cached) throw new Error("teq: no cache directory (neither TEQ_CACHE_DIR nor a home is set), and TEQ is not set")
  const coursier = coursierFile(coursierCache(), url)
  if (coursier && sizeOf(coursier) === size && sha1Of(coursier) === sha1) {
    await place(cached, size, async (part) => copyFileSync(coursier, part))
    return cached
  }
  log(`teq: fetching teq ${version} for ${platform} from ${url}`)
  await place(cached, size, (part) => fetchInto(part, { url, sha1, size }))
  return cached
}

/**
 * A file placed at `file` through a partial file of its own beside it, created afresh, which
 * `write` fills: made executable and renamed into place, removed whatever happens.
 */
async function place(file, size, write) {
  mkdirSync(dirname(file), { recursive: true })
  const part = join(dirname(file), `.${basename(file)}.${randomBytes(8).toString("hex")}.part`)
  closeSync(openSync(part, "wx"))
  try {
    await write(part)
    chmodSync(part, 0o755)
    try {
      renameSync(part, file)
    } catch (err) {
      // another fetch placed it first, and Windows refuses to replace a running binary
      if (sizeOf(file) !== size) throw err
    }
  } finally {
    rmSync(part, { force: true })
  }
}

const sizeOf = (file) => statSync(file, { throwIfNoEntry: false })?.size
const sha1Of = (file) => createHash("sha1").update(readFileSync(file)).digest("hex")

const MAX_FILESIZE_EXCEEDED = 63

/**
 * The response's body written to `part` by curl, its configuration on stdin and none of the user's
 * (a `.curlrc` could add headers to the file): redirects to https alone, a body larger than the
 * pin refused, a stalled or endless transfer given up, the bytes refused unless they are the
 * pinned ones.
 */
async function fetchInto(part, { url, sha1, size }) {
  refuseCleartext(url)
  const config = [
    `url = ${quoted(url)}`,
    `output = ${quoted(part)}`,
    'write-out = "%{http_code}"',
    "silent",
    "show-error",
    "location",
    'proto-redir = "=https"',
    `max-filesize = ${size}`,
    "connect-timeout = 15",
    "speed-limit = 1024",
    "speed-time = 60",
    "max-time = 1800",
  ]
  const curl = await run("curl", ["--disable", "--config", "-"], config.join("\n") + "\n")
  if (curl.code === MAX_FILESIZE_EXCEEDED) throw new Error(`teq: ${url} has more than the ${size} bytes the lock pins: refused`)
  if (curl.code !== 0) throw new Error(`teq: GET ${url} failed: ${curl.stderr.trim().split("\n").at(-1) || `curl exited ${curl.code}`}`)
  const status = curl.stdout.trim()
  if (status && status !== "000" && status !== "200") throw new Error(`teq: GET ${url} answered ${status}`)
  const [got, digest] = [sizeOf(part), sha1Of(part)]
  if (got !== size || digest !== sha1) throw new Error(`teq: ${url} has sha1 ${digest} and ${got} bytes where the lock pins ${sha1} and ${size} bytes: refused`)
}

function run(command, args, input) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, { stdio: ["pipe", "pipe", "pipe"] })
    let [stdout, stderr] = ["", ""]
    child.stdout.on("data", (data) => (stdout += data))
    child.stderr.on("data", (data) => (stderr += data))
    child.on("error", (err) => reject(new Error(`teq: cannot run ${command}: ${err.message}`)))
    child.on("close", (code) => resolve({ code, stdout, stderr }))
    child.stdin.on("error", () => {})
    child.stdin.end(input)
  })
}

/** Plain http reaches this machine and no other. */
function refuseCleartext(url) {
  if (url.startsWith("https://")) return
  const host = URL.canParse(url) && url.startsWith("http://") ? new URL(url).hostname.replace(/^\[|\]$/g, "") : ""
  const loopback = host === "localhost" || host === "::1" || (isIP(host) === 4 && host.startsWith("127."))
  if (!loopback) throw new Error(`teq: ${url} is not https, which teq fetches over from every host but this one`)
}

/** A string as curl's configuration file takes it. */
function quoted(s) {
  return `"${s.replace(/[\\"]/g, "\\$&").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t")}"`
}
