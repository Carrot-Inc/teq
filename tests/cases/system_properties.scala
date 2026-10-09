//> using platform jvm
// os: unix
// What the JDK says of the machine, against the JDK: `os.name` and `os.arch` as `uname` names the
// kernel and the machine in the JDK's words (Linux, Mac OS X; amd64, x86_64, aarch64), the
// separators, the line separator, the temporary directory and the home as the JDK takes them,
// `availableProcessors` within what `getconf` counts (a container's quota counts fewer); each
// compared rather than printed, so that the expectation holds on any machine of these.
object Main:
  def run(cmd: String*): String =
    val p = new ProcessBuilder(cmd*).start()
    val out = new String(p.getInputStream.readAllBytes(), "UTF-8").trim
    p.waitFor()
    out

  def show(name: String)(f: => Any): Unit =
    try println(name + ": " + String.valueOf(f))
    catch case e: Exception => println(name + ": " + e.getClass.getName + ": " + e.getMessage)

  def main(args: Array[String]): Unit =
    val kernel = run("uname", "-s")
    val machine = run("uname", "-m")
    val name = System.getProperty("os.name")
    val arch = System.getProperty("os.arch")
    show("os.name")(name == (if kernel == "Darwin" then "Mac OS X" else kernel))
    val expectedArch = machine match
      case "x86_64" => if kernel == "Darwin" then "x86_64" else "amd64"
      case "arm64" | "aarch64" => "aarch64"
      case other => other
    show("os.arch")(arch == expectedArch)
    show("file.separator")(System.getProperty("file.separator"))
    show("path.separator")(System.getProperty("path.separator"))
    show("line.separator")(System.getProperty("line.separator") == "\n" && System.lineSeparator() == "\n")
    show("user.dir")(System.getProperty("user.dir") == run("pwd"))
    show("user.home")(System.getProperty("user.home") == System.getenv("HOME"))
    show("java.io.tmpdir")(
      if kernel == "Linux" then System.getProperty("java.io.tmpdir") == "/tmp"
      else java.nio.file.Files.isDirectory(java.nio.file.Paths.get(System.getProperty("java.io.tmpdir"))))
    show("absent")(System.getProperty("teq.no.such.property", "fallback"))
    val processors = Runtime.getRuntime.availableProcessors()
    show("processors")(processors >= 1 && processors <= run("getconf", "_NPROCESSORS_ONLN").toInt)
