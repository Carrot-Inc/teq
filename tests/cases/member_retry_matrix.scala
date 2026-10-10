// The acceptance rows of a member's retry on its qualifier (dotty's `tryWithImplicitOnQualifier`
// over `FunProto`'s argument cache): an argument the member's application typed with an error
// short of one inside it (a lambda's result, an `if`'s branches, a named argument's value) is
// typed again for the retry; one it typed clean is adapted to the extension's formal as typed, its
// own type variables keeping their instances and an enclosing inference's constrained again; a
// spliced array, an eta-expanded method, a by-name argument and an operand evaluated ahead of the
// receiver are adapted as any other.
import scala.language.implicitConversions
class L1 { def f(x: () => String): String = "member" }
extension (b: L1) def f(x: () => Int): String = "extension " + x()
class I1 { def f(x: Int): String = "member" }
extension (b: I1) def f(x: String): String = "extension " + x
class N1 { def f(x: Int, y: Int): String = "member" }
extension (b: N1) def f(y: String, x: String): String = x + y
class B1[A] { def f(x: List[A], y: String): Int = 0 }
extension [A](b: B1[A]) def f(x: List[A], y: Int): Int = y
def outer[A](g: B1[A] => Int): Int = g(new B1[A])
class G1 { def g[T](x: List[T], y: String): Int = 0 }
extension (c: G1) def g[T](x: List[T], y: Int): List[T] = x
class S1 { def f(xs: Int*): String = "member" }
extension (b: S1) def f(xs: String*): String = "extension " + xs.mkString(",")
class E1 { def f(x: Int, y: Int): String = "member" }
extension (b: E1) def f(x: Int => Int, y: String): String = "extension " + x(2)
def inc(x: Int): Int = x + 1
class Bn { def f(x: => Int, y: String): Int = 0 }
extension (b: Bn) def f(x: => Int, y: Int): Int = x + x + y
class X1 { def f(x: Int): String = "member" }
class Y1 { def f(x: String): String = "conversion" }
given Conversion[X1, Y1] = _ => Y1()
extension (b: X1) def f(x: String): String = "extension"
class H1 { def +:(x: String): String = "member " + x }
class H2 { def +:(x: Int): String = "conversion " + x }
given Conversion[H1, H2] = _ => H2()
class P1 { def f(x: Long, y: String): String = "member" }
extension (b: P1) def f(x: Int, y: Int): String = "extension " + x
@main def run(): Unit =
  def arg(s: String): String = { println(s); s }
  println(L1().f(() => 1))
  println(I1().f(if true then "a" else "b"))
  println(I1().f(x = "a"))
  println(N1().f(x = arg("x"), y = arg("y")))
  println(outer(b => b.f(List(1), 3)))
  println(G1().g(List(1, 2), 3).map(_ * 2))
  val xs = Array("s", "t")
  println(S1().f(xs*))
  println(E1().f(inc, "s"))
  var n = 0
  println(Bn().f({ n += 1; n }, 10))
  println(n)
  println(X1().f("s"))
  val i: Int = 4
  println(P1().f(i, 1))
  def one(): Int = { println("one"); 1 }
  def mk(): H1 = { println("mk"); H1() }
  println(one() +: mk())
