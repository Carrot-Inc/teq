// teq: --target jvm --std=scala-library
// A bound or an argument reaching `caps.Pure` holds its argument without capture checking's `dropPure` (which reads
// the bound's type symbol, an alias's identity teq's types do not keep): an enclosing type parameter, an alias or an
// abstract member bounded by `Pure`, a sibling instantiated with `Pure`, an argument or a bound written as an alias,
// each as before this part.
import caps.Pure

final class Token extends Pure { def value: Int = 7 }

object Method:
  def bad[A <: Pure](n: Int): A =
    def id[B <: A](b: B): A = b
    id[Int](n)

object Nested:
  class C[A <: Pure]:
    class D[B <: A](val b: B):
      def get: A = b
    def bad: A = new D[Int](1).get

object Aliased:
  type P = Pure
  class C[A <: P]
  type X = C[Int]

trait Member:
  type A <: Pure
  class C[B <: A]
  type X = C[Int]

object Twin:
  def bad[A](n: Int): A =
    def id[B <: A](b: B): A = b
    id[Int](n)
object Instantiated:
  class C[A, B <: A](val b: B):
    def get: A = b
  val p: Pure = new C[Pure, Int](1).get
  def f[A, B <: A](b: B): A = b
  val q: Pure = f[Pure, Int](1)
object AliasArgument:
  type P = Pure
  trait Foo
  class C[A, B <: A]
  type X = C[P, Int]
  class D[A <: Pure, B <: A](val b: B):
    def get: A = b
  val p: Pure = new D[P, Int](1).get
  class E[A <: P & Foo]
  type Y = E[Foo]
object Applied:
  class C[A <: Pure, B <: A]
  def bad[A <: Pure](x: C[A, Int]): Int = 1
// expect: 13:5: error: type argument Int does not conform to upper bound A
// expect: 19:18: error: type argument Int does not conform to upper bound A
// expect: 24:14: error: type argument Int does not conform to upper bound Pure
// expect: 29:14: error: type argument Int does not conform to upper bound Member.this.A
// expect: 34:5: error: type argument Int does not conform to upper bound A
// expect: 38:17: error: type argument Int does not conform to upper bound Pure
// expect: 40:17: error: type argument Int does not conform to upper bound Pure
// expect: 45:17: error: type argument Int does not conform to upper bound Pure
// expect: 48:17: error: type argument Int does not conform to upper bound Pure
// expect: 50:14: error: type argument Foo does not conform to upper bound Pure & Foo
// expect: 53:30: error: type argument Int does not conform to upper bound A
// expect: 11 errors found
