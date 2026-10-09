// An extension takes a member call whose first argument list the member cannot take, as scalac's
// `tryInsertImplicitOnQualifier` has it: repeated arguments that are not the member's element
// type, a polymorphic function literal where the member takes none, a type parameter whose
// bound the argument is outside of, a value of the wrong type, a function literal of an untyped
// parameter against a member that takes no function, an array of the wrong element spliced, two
// instance creations that fix a type parameter apart (each typed without a variable in it, and
// reused as it is on every target). A member the arguments fit keeps the call, a type parameter
// taking what they give. The retry
// tries the arguments as written where the member took them as one tuple, and the arguments of
// a named one the member has no parameter of, which its application never typed; it applies the
// extension to the arguments as the member's application typed them, dotty's `FunProto` cache:
// a literal typed against the member's `Long` is a `Long` to the extension too, a closure is
// converted to a trait with one abstract method, a context function kept around a value, a
// named argument's value is adapted on its own. An infix operand's tuple is one argument to a
// unary member and the arguments of one of more parameters. A conversion of the receiver is
// taken where no extension is selected.
import scala.compiletime.erasedValue
import scala.language.implicitConversions
class Box[T]
class A { def f(xs: Int*): String = "member" }
class B { def f(n: Int, xs: Int*): String = "member" }
class E { def f(x: Int => Int): String = "member" }
class F { def f[T <: String](x: T): String = "member" }
class I { def f(xs: Seq[Int]): String = "member" }
class G { def f[T](x: Box[T], y: Box[T]): String = "member" }
extension (a: A) def f(xs: String*): String = "extension " + xs.mkString(",")
extension (b: B) def f(n: Int, xs: String*): String = "extension " + n + xs.mkString(",")
extension (e: E) def f(x: [T] => T => String): String = "extension " + x(3)
extension (f: F) def f(x: Int): String = "extension " + x
extension (i: I) def f(xs: Seq[String]): String = "extension " + xs.head
extension (g: G) def f(x: Box[Int], y: Box[String]): String = "extension"
trait Sam { def apply(x: Int): Int }
class S { def f[T <: (Int => Int) | String](x: T): String = "member" }
class V { def f[T](x: T, y: Seq[T]): String = "member" }
class W { def f[T <: AnyRef](x: T): String = "member" }
extension (s: S) def f(x: Int => String): String = "extension " + x(3)
extension (v: V) def f(x: Int, y: Seq[String]): String = "extension"
extension (w: W) def f(x: Int => String): String = "extension"
class Ctx
given Ctx = Ctx()
class X { def f(a: (Int, Int)): String = "member" }
class Z { def f(a: Int): String = "member" }
class Cx { def f(a: Ctx ?=> Int, b: Int): String = "member" }
extension (x: X) def f(a: (String, Int), b: Int): String = "extension " + a._1 + b
extension (z: Z) def f(g: Sam): String = "extension " + g(4)
extension (c: Cx) def f(a: Ctx ?=> Int, b: String): String = { val v: Int = a; "extension " + v + b }
class Lw { def f(a: Long, b: Int): String = "member" }
extension (l: Lw) inline def f[A](a: A, b: String): String =
  inline erasedValue[A] match
    case _: Long => "extension Long " + b
    case _: Int => "extension Int " + b
    case _ => "extension other " + b
class Nf { infix def f(a: Int): String = "member" }
extension (n: Nf) infix def f(a: (String, Int)): String = "extension " + a._1 + a._2
class Tf { infix def f(a: (Int, Int)): String = "member" }
extension (t: Tf) infix def f(a: String, b: Int): String = "extension " + a + b
class Cv { def f(a: Int): String = "member" }
class Dv { def f(a: String): String = "conversion " + a }
given Conversion[Cv, Dv] with
  def apply(c: Cv): Dv = Dv()
class Nv { def f(a: Int): String = "member" }
extension (n: Nv) def f(a: String): String = "extension " + a
@main def run(): Unit =
  println(A().f("x", "y"))
  println(A().f(1, 2))
  println(B().f(1, "x"))
  println(E().f([T] => (x: T) => "e" + x))
  println(F().f(1))
  val s = Seq("i")
  println(I().f(s))
  println(V().f(1, Seq("v")))
  println(W().f((x: Int) => "w"))
  println(S().f((x: Int) => x))
  val ints = Array(1, 2)
  println(A().f(ints*))
  println(X().f(("x", 1), 0))
  println(X().f(a = ("y", 1), b = 2))
  println(Z().f((x: Int) => x * 2))
  println(Cx().f(1, "c"))
  println(Lw().f(1, "w"))
  println(Nf() f ("n", 1))
  println(Tf().f("t", 2))
  println(Cv().f("v"))
  println(Nv().f(a = "n"))
  println(Z().f(x => x * 3))
  println(G().f(new Box[Int], new Box[String]))
