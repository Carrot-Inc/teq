// The JDK's file members the lean std declares without their Java varargs (`LinkOption*`,
// `String*`): the products' calls pass them empty, as scalac's do.
// teq's own JavaScript builds have no file system; scalac's regeneration from
// teq's TASTy runs them on the JVM.
import java.nio.file.{Files, Path, Paths}

object JavaVarargsFiles {
  def describe(s: String): String =
    val path = Path.of(s)
    val kind =
      if Files.isDirectory(path) then "dir"
      else if Files.isRegularFile(path) then "file"
      else if Files.exists(path) then "other"
      else "none"
    val dated = Files.exists(Paths.get(s)) && Files.getLastModifiedTime(path).toMillis > 0
    s"$kind $dated"
  def main(args: Array[String]): Unit =
    println(describe("/"))
    println(describe("/no/such/file"))
}
