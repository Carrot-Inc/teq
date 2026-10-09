// node bench/app/export-build.mjs <build.json> <teq> <out dir> [<teq argument>...]: the application's export build,
// the file sbt-teq's teqExport writes, built by <teq> the way the gate's application lines build it: its
// sources, class path and flags (and the arguments after the out dir, `--threads 4` say), split into <out dir>, in
// the root the file names. Prints one line (the root, the exit code, the error
// or total line of --time) and exits with teq's code.
import { readFileSync, mkdirSync } from "node:fs"
import { spawnSync } from "node:child_process"
import { resolve } from "node:path"
const [file, teq, out, ...extra] = process.argv.slice(2)
const d = JSON.parse(readFileSync(file, "utf-8"))
const root = resolve(file, "..", d.root ?? "../../..")
const flags = [...(d.maxInlines ? ["--max-inlines", String(d.maxInlines)] : []), ...(d.strictEquality ? ["--strict-equality"] : []), ...(d.kindProjector ? ["--kind-projector"] : []), ...(d.werror ? ["--werror"] : []), ...(d.cacheableState ?? []).flatMap((n) => ["--cacheable-state", n])]
// The words before the compiler's verb for this binary, which may be master's or the reference's from before the
// project verbs took the top level (bench/app/export-identity.sh builds both): `compiler` when `<teq> compiler --help`
// exits 0, none under the old spelling (the usage, exit 2), probed once here before the build, as
// tests/support/compiler-words.sh and bench/pairs.py probe theirs, and gone with them when bench/reference.txt
// advances past the first release that carries the top-level verbs.
const words = spawnSync(teq, ["compiler", "--help"], { stdio: "ignore", timeout: 30000 }).status === 0 ? ["compiler"] : []
const args = [...words, "build", ...d.sources, "--classpath", d.classpath.join(":"), ...flags, "--split", resolve(out), "--time", ...extra]
mkdirSync(out, { recursive: true })
const r = spawnSync(teq, args, { cwd: root, encoding: "utf-8", timeout: 300000, maxBuffer: 1 << 28 })
console.log(`root ${root}; exit ${r.status}; ` + (r.stderr ?? "").split("\n").filter((l) => / error: |errors? found|total/.test(l)).slice(-2).join(" | ").slice(0, 200))
process.exit(r.status ?? 1)
