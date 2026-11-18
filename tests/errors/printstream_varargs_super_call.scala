// expect: 15:7: error: Member method printf of mixin trait T is missing a concrete super implementation in class C.
// expect: 16:7: error: Member method format of mixin trait U is missing a concrete super implementation in class D.
// expect: 17:7: error: Member method printf of mixin trait V is missing a concrete super implementation in class E.
// expect: 3 errors found
// A trait's super call of the JDK's PrintStream.printf or format: the trait's super accessor is a Scala member
// (a Seq after ElimRepeated), the JDK's member behind the trait takes an array, so scalac's ResolveSuper finds
// no implementation for it in the class that mixes the trait in. With an override, of format, without one.
import java.io.*
trait T extends PrintStream:
  override def printf(f: String, a: Any*): PrintStream = { print("[T]"); super.printf(f, a*) }
trait U extends PrintStream:
  override def format(f: String, a: Any*): PrintStream = super.format(f, a*)
trait V extends PrintStream:
  def shout(s: String): PrintStream = super.printf("%s!", s)
class C(o: OutputStream) extends PrintStream(o) with T
class D(o: OutputStream) extends PrintStream(o) with U
class E(o: OutputStream) extends PrintStream(o) with V
