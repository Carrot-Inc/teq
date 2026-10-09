//> using platform jvm
// `java.nio.file.Path` as the JDK's `UnixPath` answers, and the process a program under
// `teq interp` sees: a path as its parser writes it, `relativize`, `normalize`, `startsWith`,
// `resolve`, and `toAbsolutePath` against `user.dir` (the working directory), whether or not the
// file is there; the environment is the process's.
import java.nio.file.{Path, Paths}

object Main:
  inline def truth(inline b: Boolean): String = if b then "yes" else "no"

  def show(name: String)(f: => Any): Unit =
    try println(name + ": <" + f + ">")
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def main(args: Array[String]): Unit =
    for p <- List("a//b/", "/", "//x//", "", "a/./b") do show("parsed " + p)(Paths.get(p))
    for (a, b) <- List(
        "assets" -> "assets/alpha.svg", "a/b" -> "a", "a/b" -> "a/c", "a" -> "a", "" -> "a", "a" -> "", "/a" -> "/a/b/c", "/a/b" -> "/c",
        "a/./b" -> "a/c", "a/b" -> "a/b/../c", "a" -> "..", "." -> "a", "a" -> ".", "a/.." -> "b", "a" -> "/a", ".." -> "a", "/" -> "/a",
      ) do show("relativize " + a + " " + b)(Paths.get(a).relativize(Paths.get(b)))
    for p <- List("a/./b/../c", "a/..", "../a", "/..", "/../a", "a/../..", ".", "./a", "/a/./b/../../..", "../../a/..") do
      show("normalize " + p)(Paths.get(p).normalize)
    for (a, b) <- List("abc" -> "a", "a/b" -> "a", "a/b" -> "a/b/c", "/a/b" -> "/a", "/a/b" -> "a", "/a" -> "/", "a" -> "", "" -> "", "a//b" -> "a/b/") do
      show("startsWith " + a + " " + b)(Paths.get(a).startsWith(b))
    show("startsWith a path")(Paths.get("a/b").startsWith(Paths.get("a")))
    show("resolve a path")(Paths.get("a").resolve(Paths.get("b/c")))
    show("resolve an absolute path")(Paths.get("a").resolve(Paths.get("/b")))
    show("resolve an operand as the parser writes it")(Paths.get("a").resolve("b//c/"))
    show("resolve an absolute operand so")(Paths.get("a").resolve("//x//y/"))
    // toAbsolutePath: a relative path against user.dir, there or not; an absolute one itself.
    val dir = System.getProperty("user.dir")
    show("user.dir is absolute")(Paths.get(dir).isAbsolute)
    show("user.dir is the empty path's")(Paths.get("").toAbsolutePath.toString == dir)
    for p <- List("not/there.txt", "tests", ".", "a/../b", "rel/") do
      show("toAbsolutePath " + p)(Paths.get(p).toAbsolutePath.toString.replace(dir, "<dir>"))
    show("toAbsolutePath absolute")(Paths.get("/x/y").toAbsolutePath)
    show("toAbsolutePath is absolute")(Paths.get("x").toAbsolutePath.isAbsolute)
    show("under user.dir")(Paths.get("x").toAbsolutePath.startsWith(dir))
    show("relativized back")(Paths.get(dir).relativize(Paths.get("tests/cases").toAbsolutePath))
    // The process's environment: a variable every shell sets, one none does.
    show("PATH set")(System.getenv("PATH") != null)
    show("PATH in sys.env")(sys.env.contains("PATH"))
    show("PATH in the map")(System.getenv().containsKey("PATH"))
    show("unset")(System.getenv("TEQ_UNSET_VARIABLE_8271"))
    show("user.dir in sys.props")(sys.props("user.dir") == dir)
    // Read when the program runs, never folded into a constant at compile time.
    show("PATH through an inline argument")(truth(sys.env.contains("PATH")))
    show("user.dir through an inline argument")(truth(System.getProperty("user.dir") != null))
