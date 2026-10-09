// Bindings to node built-ins: named, default, namespace and value imports.

@jsImport("node:path", "join")
def join(parts: String*): String

@jsImport("node:util", "format")
def format(template: String, args: Any*): String

@jsImport("node:util", "inspect")
def inspect(value: Any): String

@jsImport("node:path", "basename")
def basename(path: String): String

// No overloading, but two definitions may bind the same export; it is imported once.
@jsImport("node:path", "basename")
def basenameWithout(path: String, suffix: String): String

@jsImport("node:path", "relative")
def relative(from: String)(to: String): String

@jsImport("node:path", "extname") def extname(path: String): String

@jsImport("node:path", "sep") val sep: String

@jsImport("node:path", "delimiter")
def delimiter: String

@jsImport("node:path", "normalize")
val normalize: String => String

@jsImport("node:path/posix", "isAbsolute")
def isAbsolute(path: String): Boolean

@jsImport("node:os", "default")
val os: Any

@jsImport("node:path", "*")
val path: Any

@jsImport("node:url", "URL")
val URL: Any

@jsImport("node:url", "pathToFileURL")
def pathToFileURL(path: String): Any

@main def main(): Unit =
  println(join("a", "b", "c.txt"))
  println(join("only"))
  println(join())
  val parts = List("usr", "local", "bin")
  println(join(parts*))
  println(join(Vector("x", "y")*))
  println(join(parts.map(p => p.toUpperCase)*))
  // as a function value a vararg import takes a Seq, which is spread as well
  println(List(List("x", "y"), List("z")).map(join))

  println(format("%s has %d items", "cart", 3))
  println(format("no arguments"))
  val args: List[Any] = List("spliced", 2, true)
  println(format("%s %d %s", args*))
  println(inspect(js.obj("a" -> 1, "b" -> "two")))

  println(basename("/tmp/report.pdf"))
  println(basenameWithout("/tmp/report.pdf", ".pdf"))
  println(relative("/data/a/b")("/data/a/c/d"))
  println(List("a.txt", "b.tar.gz", "c").map(extname))

  println(sep)
  println(delimiter)
  println(normalize("/a//b/../c/"))
  println(isAbsolute("/a"))
  println(isAbsolute("a"))

  println(js.typeOf(js.call(os, "platform")))
  println(js.typeOf(js.get(os, "cpus")))
  println(js.call(path, "dirname", "/a/b/c.txt"))
  println(js.get(path, "sep") == sep)

  val url = js.construct(URL, "https://example.com/docs/index.html?lang=en#top")
  println(js.get(url, "hostname"))
  println(js.get(url, "pathname"))
  println(js.call(js.get(url, "searchParams"), "get", "lang"))
  println(js.get(pathToFileURL("/tmp/x y.txt"), "href"))
