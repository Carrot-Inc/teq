// A file stream made inside a callback that never runs: the program builds and runs on every back end (JavaScript
// has no file streams: the failure is the call's, when it runs).
import java.io.FileOutputStream
object Probe:
  def main(args: Array[String]): Unit =
    List.empty[String].foreach { name =>
      val s = new FileOutputStream(name)
      s.write(65)
      s.close()
    }
    println("ok")
