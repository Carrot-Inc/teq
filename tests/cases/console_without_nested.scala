// Console.withOut captures whichever stream it names (System.out's own too), nested and restored after a throw;
// Console.out keeps its first binding after System.setOut; a closure made inside prints where it runs.
import java.io.*
object Probe:
  def main(args: Array[String]): Unit =
    val saved = System.out
    val initial = scala.Console.out
    val b = new ByteArrayOutputStream
    val p = new PrintStream(b)
    System.setOut(p)
    saved.println("console-initial=" + (scala.Console.out eq initial))
    scala.Console.withOut(p) {
      print("part")
      scala.Console.withOut(saved) { print("inner") }
      try scala.Console.withOut(p) { print("throw"); throw new RuntimeException("x") }
      catch case _: RuntimeException => ()
      println("end")
      saved.println("console-p=" + (scala.Console.out eq p))
      scala.Console.out.print("direct")
    }
    System.setOut(saved)
    saved.println("capture=" + b.toString("UTF-8").replace("\n", "|"))
    var later: () => Unit = () => ()
    scala.Console.withOut(p) { later = () => print("later") }
    later(); println("-done")
