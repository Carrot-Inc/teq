// System.setOut before Console's first use: Console binds the moved stream at that use (scala.Console's `out` is a DynamicVariable
// whose first value is System.out as it stands then), and Predef's println() is Console's, so the prelude's println(), Console's,
// Console.out's and System.out's all reach the subclass. The expected lines are scalac's on the JDK.
import java.io.{ByteArrayOutputStream, OutputStream, PrintStream}
class Capture(o: OutputStream) extends PrintStream(o):
  override def println(): Unit = super.print("zero;")
  override def println(x: Any): Unit = super.print("value;")
object Probe:
  def main(args: Array[String]): Unit =
    val saved = System.out
    val b = new ByteArrayOutputStream
    System.setOut(new Capture(b))
    println()
    scala.Console.println()
    scala.Console.out.println()
    System.out.println()
    System.setOut(saved)
    saved.println(b.toString("UTF-8"))
