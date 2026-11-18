// expect: 12:7: error: class C inherits conflicting members: printf in trait T1 and printf in trait T2; declare an override in class C
// expect: 1 error found
// scalac: "error overriding method printf in trait T1 ..; method printf in trait T2 .. class C inherits conflicting
// members: method printf in trait T1 .. and method printf in trait T2 .. (Note: this can be resolved by declaring
// an override in class C.)": its ElimRepeated takes `override` off a member that overrides a Java varargs method
// (ElimRepeated.scala 69-74), so the two overrides of the JDK's printf are unrelated members to RefChecks.
import java.io.*
trait T1 extends PrintStream:
  override def printf(f: String, a: Any*): PrintStream = { print("[T1]"); this }
trait T2 extends PrintStream:
  override def printf(f: String, a: Any*): PrintStream = { print("[T2]"); this }
class C(o: OutputStream) extends PrintStream(o) with T1 with T2
