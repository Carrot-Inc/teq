//> using platform jvm
// os: unix
// `ProcessBuilder` against the JDK: a child's three streams piped (the default), inherited,
// discarded, merged (`redirectErrorStream`), read from a file, written and appended to files; the
// stream of a redirect that is no pipe reads nothing and refuses a write; the redirects' texts and
// equality; what the interpreter printed comes before an inheriting child's output. The children
// are `sh` and `printf`; the files are under target/ of the working directory, removed at the end.
import java.io.File
import java.lang.ProcessBuilder.Redirect
import java.nio.file.{Files, Paths}
import scala.jdk.CollectionConverters.*

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def text(in: java.io.InputStream): String = new String(in.readAllBytes(), "UTF-8")

  def sh(script: String): ProcessBuilder = new ProcessBuilder("sh", "-c", script)

  def main(args: Array[String]): Unit =
    val dir = Paths.get("target/process_redirects")
    if Files.exists(dir) then Files.walk(dir).iterator.asScala.toList.reverse.foreach(Files.delete)
    Files.createDirectories(dir)
    try run(dir)
    finally Files.walk(dir).iterator.asScala.toList.reverse.foreach(Files.delete)

  def run(dir: java.nio.file.Path): Unit =
    // Piped, the default: both outputs read after the child ends, its status.
    val p = sh("printf 'out\\n'; printf 'err\\n' >&2; exit 4").start()
    show("piped out")(text(p.getInputStream))
    show("piped err")(text(p.getErrorStream))
    show("piped status")(p.waitFor())
    show("exitValue")(p.exitValue())
    // Merged: one stream, in the order written.
    val m = sh("printf 'a\\n'; printf 'b\\n' >&2; printf 'c\\n'").redirectErrorStream(true).start()
    show("merged")(text(m.getInputStream).replace("\n", "|"))
    show("merged err")(m.getErrorStream.read())
    m.waitFor()
    // Inherited: the child writes to this program's stdout, after what was printed before it.
    println("before the inheriting child")
    sh("printf 'from the child\\n'").redirectOutput(Redirect.INHERIT).start().waitFor()
    println("after the inheriting child")
    val i = sh("true").inheritIO()
    show("inheritIO")(List(i.redirectInput(), i.redirectOutput(), i.redirectError()).mkString(","))
    // Discarded: the stream of no pipe reads -1 at once, available 0, and a write is refused.
    val d = sh("printf 'lost\\n'").redirectOutput(Redirect.DISCARD).redirectInput(Redirect.INHERIT).start()
    d.waitFor()
    show("discarded read")(d.getInputStream.read())
    show("discarded available")(d.getInputStream.available())
    show("no pipe write")({ d.getOutputStream.write(1); "written" })
    // Files: written, appended, read.
    val out = dir.resolve("out.txt").toFile
    sh("printf 'first\\n'").redirectOutput(out).start().waitFor()
    sh("printf 'second\\n'").redirectOutput(Redirect.appendTo(out)).start().waitFor()
    show("appended")(Files.readString(out.toPath).replace("\n", "|"))
    sh("printf 'third\\n'").redirectOutput(Redirect.to(out)).start().waitFor()
    show("truncated")(Files.readString(out.toPath).replace("\n", "|"))
    val r = new ProcessBuilder("cat").redirectInput(out).start()
    show("from a file")(text(r.getInputStream).replace("\n", "|"))
    r.waitFor()
    val e = sh("printf 'to err\\n' >&2").redirectError(dir.resolve("err.txt").toFile).start()
    e.waitFor()
    show("err to a file")(Files.readString(dir.resolve("err.txt")).trim)
    show("missing input file")(new ProcessBuilder("cat").redirectInput(dir.resolve("absent").toFile).start())
    show("output under a missing directory")(sh("true").redirectOutput(dir.resolve("no/such/file").toFile).start())
    // The redirects themselves.
    show("redirects")(List(Redirect.PIPE, Redirect.INHERIT, Redirect.DISCARD, Redirect.to(new File("a b")), Redirect.appendTo(new File("x")), Redirect.from(new File("y"))).mkString(" | "))
    show("types")(List(Redirect.PIPE, Redirect.INHERIT, Redirect.DISCARD, Redirect.to(new File("a")), Redirect.appendTo(new File("x")), Redirect.from(new File("y"))).map(_.`type`()).mkString(" "))
    show("files")(List(Redirect.PIPE, Redirect.DISCARD, Redirect.to(new File("a"))).map(_.file()).mkString(" "))
    show("equal")(Redirect.to(new File("a")) == Redirect.to(new File("a")))
    show("unequal")(Redirect.to(new File("a")) == Redirect.appendTo(new File("a")))
    show("input to a file")(new ProcessBuilder("cat").redirectInput(Redirect.to(new File("a"))))
    show("output from a file")(new ProcessBuilder("cat").redirectOutput(Redirect.from(new File("a"))))
