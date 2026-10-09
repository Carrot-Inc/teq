// demo.Labels from the labels file: `node generate-labels.mjs <labels file> <source directory>`
// writes <source directory>/demo/Labels.scala, and leaves it alone when its text is already that.
// The generator of browserdemo and mirrored, for sbt and for `teq` alike.
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs"
import { join } from "node:path"

const [labels, directory] = process.argv.slice(2)
if (!labels || !directory) {
  console.error("usage: node generate-labels.mjs <labels file> <source directory>")
  process.exit(2)
}
const footer = readFileSync(labels, "utf-8").trim()
const file = join(directory, "demo", "Labels.scala")
const text = `package demo\n\nobject Labels:\n  val footer: String = "${footer}"\n`
if (!existsSync(file) || readFileSync(file, "utf-8") !== text) {
  mkdirSync(join(directory, "demo"), { recursive: true })
  writeFileSync(file, text)
}
