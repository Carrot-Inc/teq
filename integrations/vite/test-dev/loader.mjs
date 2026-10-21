// The plugin's teq process, binary resolution and export are replaced under test-dev.mjs: the
// three modules index.js imports by these names come from this directory instead.
const replaced = new Set(["./teq.js", "./binary.js", "./description.js"])
export async function resolve(specifier, context, next) {
  if (replaced.has(specifier) && context.parentURL?.endsWith("/index.js")) return { url: new URL(specifier, import.meta.url).href, shortCircuit: true }
  return next(specifier, context)
}
