// A member that applies to its first argument list keeps its errors past that boundary: an
// extension of the name takes over only from a member that cannot take the arguments, as scalac's
// `tryInsertImplicitOnQualifier` has it. A missing given, an inline expansion's `error`, a
// lambda's body, a later argument list, a block ending in a lambda typed against the parameter, a
// literal of an untyped parameter, which is its shape alone to the test (`x => "text"`). A
// method's type parameter is a variable to the test (`Box[A]` takes a `Box[Int]`), type arguments
// written leave no extension the call, and an argument typed against its parameter with an error
// inside it (`("x", 1)` for an `(Int, Int)`, a sequence spliced into an `Int*`, an array built
// there, a polymorphic function literal's body, a block that ends in one, a named argument's
// lambda body, a SAM's lambda body, an element of a tuple for a parameter left missing) is its
// own; and a retry applies the extension to the arguments as the member's application typed
// them, dotty's `FunProto` cache: no context function is put around one, a literal widened to
// the member's `Long` stays a `Long` (alone, in a `List(1)`, a `Tuple2(1, 2)`, a block), an infix
// operand's tuple stays one argument to a unary member, and an extension selected on the
// qualifier whose application fails opens no
// conversion search.
// expect: 88:19: error: no given instance of type Evidence was found for parameter x$2
// expect: 89:11: error: member refused
// expect: 90:22: error: value toLowerCase is not a member of Int
// expect: 91:20: error: type mismatch: found String, required Int
// expect: 92:55: error: type mismatch: found String, required Int
// expect: 93:22: error: type mismatch: found String, required Int
// expect: 94:30: error: no given instance of type Evidence was found
// expect: 95:31: error: type mismatch: found String, required Int
// expect: 96:11: error: member refused
// expect: 97:22: error: type mismatch: found String, required Int
// expect: 98:18: error: type mismatch: found String, required Int
// expect: 99:21: error: type mismatch: found String
// expect: 100:17: error: type mismatch: found String, required T
// expect: 101:38: error: type mismatch: found String
// expect: 102:39: error: type mismatch: found String
// expect: 103:55: error: type mismatch: found String, required Int
// expect: 104:29: error: type mismatch: found String
// expect: 105:33: error: type mismatch: found String, required Int
// expect: 106:17: error: type mismatch: found String, required Int
// expect: 107:18: error: type mismatch: found String, required Int
// expect: 108:21: error: type mismatch: found String, required Int
// expect: 109:27: error: type mismatch: found String, required Int
// expect: 110:32: error: type mismatch: found String, required Int
// expect: 111:25: error: type mismatch: found String, required Int
// expect: 112:18: error: type mismatch: found String, required Int
// expect: 113:17: error: type mismatch: found String, required Int
import scala.compiletime.error
import scala.language.implicitConversions
trait Evidence
class A { def f(x: Int)(using Evidence): String = "member" }
class B { inline def f(x: Int): String = error("member refused") }
class C { def f(x: Int => Int): String = "member" }
class D { def f(x: Int)(y: Int): String = "member" }
class E { def f(x: Int => Int): String = "member" }
extension (a: A) def f(x: Any): String = "extension"
extension (b: B) def f(x: Any): String = "extension"
extension (c: C) def f(x: String => String): String = "extension"
extension (d: D) def f(x: Int)(y: String): String = "extension"
extension (e: E) def f(x: String => String): String = x("x")
class G { def f(x: Int => Int): String = "member" }
extension (g: G) def f(x: String => String): String = "extension"
class Box[T]
class H { def f[T](x: Box[T])(using Evidence): String = "member" }
class I { def f[T](x: Box[T])(y: Int): String = "member" }
class J { inline def f[T](x: Box[T]): String = error("member refused") }
class K { def f[T](x: T): String = "member" }
class L { def f(x: (Int, Int)): String = "member" }
class M { def f(xs: Int*): String = "member" }
extension (h: H) def f(x: Box[Int]): String = "extension"
extension (i: I) def f(x: Box[Int])(y: String): String = "extension"
extension (j: J) def f(x: Box[Int]): String = "extension"
extension (k: K) def f[T](x: String): String = "extension"
extension (l: L) def f(x: (String, Int)): String = "extension"
extension (m: M) def f(xs: String*): String = "extension"
class N { def f(x: [T] => T => Seq[Int]): String = "member" }
class O { def f(x: [T] => T => Option[Int]): String = "member" }
class P { def f(x: [T] => T => Int): String = "member" }
extension (n: N) def f(x: [T] => T => Seq[String]): String = "extension"
extension (o: O) def f(x: [T] => T => Option[String]): String = "extension"
extension (p: P) def f(x: [T] => T => String): String = "extension"
trait Sam { def apply(x: Int): Array[Int] }
class Ctx
class Q { def f(a: Sam): String = "member" }
class R { def f(a: Int => Int): String = "member" }
class S { def f(a: Ctx ?=> Int): String = "member" }
class U { def f(a: (Int, Int), b: Int): String = "member" }
extension (q: Q) def f(a: Int => Array[String]): String = "extension"
extension (r: R) def f(a: Int => String): String = "extension"
extension (s: S) def f(a: Ctx ?=> String): String = "extension"
extension (u: U) def f(a: (String, Int)): String = "extension"
@main def run(): Unit =
  println(A().f(1))
  println(B().f(1))
  println(C().f(x => x.toLowerCase))
  println(D().f(1)("s"))
  println(E().f({ println("argument"); (s: String) => s.toUpperCase }))
  println(G().f(x => "text"))
  println(H().f(new Box[Int]))
  println(I().f(new Box[Int])("x"))
  println(J().f(new Box[Int]))
  println(K().f[Int]("x"))
  println(L().f(("x", 1)))
  println(M().f(Seq("x")*))
  println(M().f(Array("s")*))
  println(N().f([T] => (x: T) => Seq("s")))
  println(O().f([T] => (x: T) => Some("s")))
  println(P().f({ println("effect"); [T] => (x: T) => "s" }))
  println(Q().f((x: Int) => Array("s")))
  println(R().f(a = (x: Int) => "s"))
  println(S().f("s"))
  println(U().f(("s", 1)))
  println(V1().f(1, "s"))
  println(V2().f(List(1), "s"))
  println(V3().f(Tuple2(1, 2), "s"))
  println(V1().f({ 1 }, "s"))
  println(W() f ("s", 1))
  println(X().f("s"))
class V1 { def f(a: Long, b: Int): String = "member" }
class V2 { def f(a: List[Long], b: Int): String = "member" }
class V3 { def f(a: (Long, Long), b: Int): String = "member" }
extension (v: V1) def f(a: Int, b: String): String = "extension"
extension (v: V2) def f(a: List[Int], b: String): String = "extension"
extension (v: V3) def f(a: (Int, Int), b: String): String = "extension"
class W { infix def f(a: (Int, Int)): String = "member" }
extension (w: W) infix def f(a: String, b: Int): String = "extension"
class X { def f(a: Int): String = "member" }
class Y { def f(a: String): String = "conversion" }
given Conversion[X, Y] with
  def apply(x: X): Y = Y()
extension (x: X) def f(a: Boolean): String = "extension"
