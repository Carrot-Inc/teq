// expect: 24:10: error: invalid new prefix  <: F cannot replace (f :  <: F) in type f.A
// expect: 26:13: error: (c: Ctx, x: c.T): c.T is an illegal function type because it has inter-parameter dependencies
// expect: 27:38: error: Implementation restriction: Expected result type (c: Ctx) ?=> (D) ?=> c.T
// expect: 28:37: error: Implementation restriction: Expected result type (c: Ctx) => (D) ?=> c.T
// expect: 32:20: error: type mismatch: found left.T, required right.T
// expect: 34:42: error: type mismatch: found (c: Ctx) => List[c.T] => Int, required Ctx => List[Any] => Int
// expect: 36:32: error: type mismatch: found (c: Ctx) => Box[c.T], required Ctx => Box[Any]
// expect: 37:61: error: type mismatch: found (c: Ctx) => F[c.T], required Ctx => F[?]
// A function type naming its parameters (scalac 3.8.4: the same messages, "Found: left.T,
// Required: right.T" as E007): a parameter type naming an earlier parameter, a curried context
// function type expected of a lambda or of a wrapped expression, a call's result at another
// path, a widening that the result's variance does not allow, a wildcard parameter the result
// selects through, and an invariant type constructor's application, which approximates to `Any`.
trait Ctx:
  type T
  def make: T
object IntCtx extends Ctx:
  type T = Int
  def make: Int = 7
class Box[A](val a: A)
trait D
trait F:
  type A
type G = (f: ? <: F) => f.A
@main def run(): Unit =
  val bad1: (c: Ctx, x: c.T) => c.T = ???
  val bad2: (c: Ctx) ?=> D ?=> c.T = ???
  val bad3: (c: Ctx) => D ?=> c.T = c => ???
  val fn: (c: Ctx) => c.T = c => c.make
  val left: Ctx = IntCtx
  val right: Ctx = IntCtx
  val r: right.T = fn(left)
  val f: (c: Ctx) => List[c.T] => Int = c => xs => xs.size
  val widened: Ctx => List[Any] => Int = f
  val g: (c: Ctx) => Box[c.T] = c => Box(c.make)
  val boxed: Ctx => Box[Any] = g
def unreducible[F[_]](f: (c: Ctx) => F[c.T]): Ctx => F[?] = f
