// A library: the exports are the roots, and an import binding is emitted when something reached
// uses it.
package shape

@jsImport("node:path", "basename")
def basename(path: String): String

@jsImport("node:path", "extname")
def extname(path: String): String

def describe(path: String): String = s"${basename(path)} (${extname(path)})"

@jsExport("describe")
def describeFile(path: String): String = describe(path)

@jsExport("count")
val count: Int = 1 + 2
