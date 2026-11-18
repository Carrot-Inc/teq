// An override of an override of `printf`: the plain meaning, each level calling the one above. A DIFFERENCE from
// scalac 3.8.4, which refuses both programs ("method printf of type (f: String, a: Seq[Any]): java.io.PrintStream
// needs `override` modifier", said of a member written `override`): its ElimRepeated takes the flag off a member
// overriding a Java varargs method (ElimRepeated.scala 69-74), so no spelling of a second-level override is
// accepted. The expected lines are teq's; on the JVM the override of printf itself is not supported yet.
import java.io.{ByteArrayOutputStream, OutputStream, PrintStream}
class M(o: OutputStream) extends PrintStream(o):
  override def printf(f: String, a: Any*): PrintStream = { print("[M.printf]"); super.printf(f, a*) }
class N(o: OutputStream) extends M(o):
  override def printf(f: String, a: Any*): PrintStream = { print("[N.printf]"); super.printf(f, a*) }
trait T extends PrintStream:
  override def printf(f: String, a: Any*): PrintStream = { print("[T.printf]"); this }
class C(o: OutputStream) extends M(o) with T:
  override def printf(f: String, a: Any*): PrintStream = { print("[C.printf]"); super[M].printf(f, a*) }
object Probe:
  def main(args: Array[String]): Unit =
    val b = new ByteArrayOutputStream
    new N(b).printf("%d", 1)
    (new N(b): PrintStream).printf("%d", 2)
    println(b.toString("UTF-8"))
    val c = new ByteArrayOutputStream
    new C(c).printf("%d", 3)
    (new C(c): PrintStream).format("%d", 4)
    println(c.toString("UTF-8"))
