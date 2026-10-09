//> using platform jvm
// os: unix
// What a child is given against the JDK: the arguments as written, one each (a space, an empty
// string, quotes, a backslash and non-ASCII text kept), no shell between; the environment the
// interpreter has, a variable added, changed and removed for the child alone; a cleared one, the
// program still found on the interpreter's PATH; the working directory, relative or absolute,
// and a missing one. The child is `sh`; the directory is under target/, removed at the end.
import java.io.File
import java.nio.file.{Files, Paths}

object Main:
  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f).replace(System.getProperty("user.dir"), "<dir>"))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + String.valueOf(e.getMessage).replace(System.getProperty("user.dir"), "<dir>"))

  def output(pb: ProcessBuilder): String =
    val p = pb.start()
    val text = new String(p.getInputStream.readAllBytes(), "UTF-8")
    p.waitFor()
    text

  def main(args: Array[String]): Unit =
    show("arguments")(output(new ProcessBuilder("sh", "-c", "for a in \"$@\"; do printf '<%s>' \"$a\"; done", "sh", "a b", "", "\"quoted\"", "back\\slash", "é ü 日本", "$HOME", "*")))
    show("argument count")(output(new ProcessBuilder("sh", "-c", "echo $#", "sh", "one two", "three")).trim)
    val pb = new ProcessBuilder("sh", "-c", "printf '%s|%s|%s' \"${TEQ_CASE_ADDED-unset}\" \"${TEQ_CASE_PATH_SAME-unset}\" \"${HOME-unset}\"")
    val env = pb.environment()
    show("inherited")(env.get("PATH") == System.getenv("PATH"))
    show("same map")(pb.environment() eq env)
    env.put("TEQ_CASE_ADDED", "added é")
    env.put("TEQ_CASE_PATH_SAME", if env.get("PATH") == null then "none" else "path")
    env.remove("HOME")
    show("the child's")(output(pb))
    show("the interpreter's")(System.getenv("TEQ_CASE_ADDED") == null && (System.getenv("HOME") != null) == (System.getenv("HOME") != null))
    val cleared = new ProcessBuilder("sh", "-c", "printf '%s|%s' \"${HOME-unset}\" \"${TEQ_CASE_ADDED-unset}\"")
    cleared.environment().put("TEQ_CASE_ADDED", "added")
    cleared.environment().clear()
    show("cleared")(output(cleared))
    val dir = Paths.get("target/process_environment")
    Files.createDirectories(dir)
    try
      show("relative directory")(output(new ProcessBuilder("sh", "-c", "pwd").directory(new File("target/process_environment"))).trim)
      show("absolute directory")(output(new ProcessBuilder("sh", "-c", "pwd").directory(dir.toAbsolutePath.toFile)).trim)
      show("directory")(new ProcessBuilder("sh").directory(new File("target/process_environment")).directory())
      show("missing directory")(new ProcessBuilder("sh", "-c", "pwd").directory(new File("target/process_environment/absent")).start())
    finally Files.delete(dir)
    show("command")(new ProcessBuilder("a", "b").command())
    show("command replaced")(new ProcessBuilder("a").command("c", "d").command())
    show("null command")(new ProcessBuilder(java.util.Arrays.asList("sh", null)).start())
