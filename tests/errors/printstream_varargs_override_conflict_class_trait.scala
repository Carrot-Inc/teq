// expect: 12:7: error: class C inherits conflicting members: printf in class M and printf in trait T; declare an override in class C
// expect: 1 error found
// scalac: "error overriding method printf in class M ..; method printf in trait T .. class C inherits conflicting
// members: method printf in class M .. and method printf in trait T .. (Note: this can be resolved by declaring an
// override in class C.)": its ElimRepeated takes `override` off a member that overrides a Java varargs method
// (ElimRepeated.scala 69-74), so the two overrides of the JDK's printf are unrelated members to RefChecks.
import java.io.*
class M(o: OutputStream) extends PrintStream(o):
  override def printf(f: String, a: Any*): PrintStream = { print("[M]"); super.printf(f, a*) }
trait T extends PrintStream:
  override def printf(f: String, a: Any*): PrintStream = { print("[T]"); this }
class C(o: OutputStream) extends M(o) with T
