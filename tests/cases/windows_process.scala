//> using platform jvm
// os: windows
// Windows' processes against the JDK on Windows (the coordinator's acceptance run, `teq.cmd interp`; its
// expectation is the Windows JDK's): the environment a child inherits, an entry replaced and one removed
// by keys of another case; a child's absolute working directory, with a space; the command line a child
// is given for arguments with spaces, an empty one and one ending in a backslash, as `cmd /c echo` shows
// it (an argument holding a quote is quoted otherwise by teq: its differences note); a pipe read past the
// child's end, `available` and a partial read first, then the end of the stream; a deadline missed, then
// the child destroyed and another destroyed forcibly, each status the JDK's (1: Windows has no signals);
// a copied `.cmd` file run by `cmd /c` from its directory, whose name has a non-ASCII character.
import java.lang.ProcessBuilder.Redirect
import java.nio.file.{Files, Path}
import java.util.concurrent.TimeUnit
import scala.jdk.CollectionConverters.*

object Main:
  def show(name: String)(f: => Any): Unit =
    val r =
      try f.toString
      catch case e: Exception => e.getClass.getName
    println(s"$name: $r")

  // The child's stdout, its line ends and trailing spaces left out.
  def output(pb: ProcessBuilder): String =
    val p = pb.redirectErrorStream(true).start()
    val text = new String(p.getInputStream.readAllBytes())
    p.waitFor()
    text.replace("\r\n", "\n").linesIterator.map(_.stripTrailing).mkString("|")

  def cmd(line: String*): ProcessBuilder = new ProcessBuilder((Seq("cmd", "/c") ++ line).asJava)

  def main(args: Array[String]): Unit =
    val pb = cmd("if defined PATH (echo inherited) else (echo missing)")
    val env = pb.environment()
    show("inherited")(output(pb))
    show("a key of another case")(env.get("pAtH") == env.get("PATH") && env.containsKey("path"))
    env.put("path", "C:\\teq-nowhere;" + env.get("PATH"))
    show("one PATH")(env.keySet.asScala.count(_.equalsIgnoreCase("PATH")))
    val replaced = cmd("echo %PATH%")
    replaced.environment().put("path", "C:\\teq-nowhere;" + System.getenv("PATH"))
    show("replaced")(output(replaced).startsWith("C:\\teq-nowhere;"))
    val removing = cmd("if defined TEQ_ACCEPT_X (echo set) else (echo removed)")
    removing.environment().put("TEQ_ACCEPT_X", "1")
    show("set")(output(removing))
    removing.environment().remove("teq_accept_x")
    show("removed")(output(removing))
    val noTemp = cmd("if defined TEMP (echo set) else (echo removed)")
    noTemp.environment().remove("temp")
    show("an inherited one removed")(output(noTemp))

    // cmd writes in the console's code page: the directory it prints is ASCII.
    val dir = Files.createTempDirectory("teq child")
    show("working directory")(output(cmd("cd").directory(dir.toFile)) == dir.toString)

    show("command line")(output(cmd("echo", "a b", "", "plain", "trail dir\\")))

    val echo = new ProcessBuilder("cmd", "/c", "echo hello").start()
    echo.waitFor()
    val in = echo.getInputStream
    show("available")(in.available())
    val buf = new Array[Byte](3)
    show("partial read")(in.read(buf, 0, 3))
    show("the rest")(new String(in.readAllBytes()).replace("\r", "\\r").replace("\n", "\\n"))
    show("end")(in.read())
    in.close()

    val slow = new ProcessBuilder("ping", "-n", "30", "127.0.0.1").redirectOutput(Redirect.DISCARD).start()
    show("deadline missed")(slow.waitFor(1, TimeUnit.SECONDS))
    show("alive")(slow.isAlive)
    slow.destroy()
    show("destroyed")(slow.waitFor())
    show("status kept")(slow.exitValue())
    val forced = new ProcessBuilder("ping", "-n", "30", "127.0.0.1").redirectOutput(Redirect.DISCARD).start()
    forced.destroyForcibly()
    show("destroyed forcibly")(forced.waitFor())

    val scripts = Files.createDirectories(dir.resolve("copied \u00fc"))
    Files.writeString(scripts.resolve("echoes.cmd"), "@echo off\r\necho [%~1] [%~2]\r\n")
    show("a copied .cmd")(output(cmd("echoes.cmd", "a b", "c").directory(scripts.toFile)))

    Files.walk(dir).iterator().asScala.toList.reverse.foreach(Files.delete)
    show("cleaned")(Files.exists(dir))
