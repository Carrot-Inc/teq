// Under Console.withOut the prelude's print and println call the stream's own members: print(x) its print of a
// String, println(x) its println of an object, println() its println().
import java.io.*
class Capture(b: ByteArrayOutputStream) extends PrintStream(b):
  override def print(s: String): Unit = super.print("S[" + s.replace("\n", "!") + "]")
  override def println(x: Any): Unit = super.print("O[" + x + "]")
  override def println(): Unit = super.print("N[]")
object Probe:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    scala.Console.withOut(new Capture(b)) { print("a"); println("b"); println() }
    println(b.toString("UTF-8"))
