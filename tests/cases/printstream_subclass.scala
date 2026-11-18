// java.io.PrintStream is an open class: a subclass overrides print and write and is passed where a stream is wanted.
import java.io.{OutputStream, PrintStream}
class Tee(out: OutputStream) extends PrintStream(out):
  override def println(x: String): Unit = super.println("<" + x + ">")
  override def print(i: Int): Unit = super.print("#" + i)
@main def run(): Unit =
  val t: PrintStream = new Tee(System.out)
  t.println("through the reference")
  t.print(5)
  t.println()
  t.println(6)
  t.println('c')
  t.flush()
