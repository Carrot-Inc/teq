// expect: 23:38: error: type mismatch: found Apply, required Int
// expect: 25:42: error: type mismatch: found Apply, required Int
// expect: 27:36: error: type mismatch: found Tree, required Int
// expect: 29:36: error: type mismatch: found Tree, required Int
// expect: 31:39: error: type mismatch: found Apply, required Int
// expect: 33:37: error: type mismatch: found Any, required Int
// expect: 35:38: error: type mismatch: found Apply & T, required Int
// expect: 39:40: error: type mismatch: found Apply & Holder.this.U, required Int
// expect: 43:38: error: type mismatch: found B & A, required Int
// The types scalac 3.8.4 gives the binders, by its own messages: the scrutinee's type where
// it conforms to what `unapply` takes (the generic `Same`, `Wide`), the narrower of the two,
// and their intersection with a type parameter, an abstract type or an unrelated trait.
sealed trait Tree
case class Apply(fun: Tree, args: List[Tree]) extends Tree
case class Name(s: String) extends Tree
object Narrow { def unapply(t: Apply): Option[Tree] = Some(t.fun) }
object NarrowSeq { def unapplySeq(t: Apply): Option[Seq[Tree]] = Some(t.args) }
object Same { def unapply[T](t: T): Option[T] = Some(t) }
object Wide { def unapply(t: Any): Option[Int] = Some(1) }
object Bounded { def unapply[T <: Apply](t: T): Option[T] = Some(t) }
object Heads { def unapply[T](t: List[T]): Option[T] = t.headOption }
def a(x: Tree): Unit = x match
  case b @ Narrow(_) => val i: Int = b
def b(x: Tree): Unit = x match
  case b @ NarrowSeq(_*) => val i: Int = b
def c(x: Tree): Unit = x match
  case b @ Same(_) => val i: Int = b
def d(x: Tree): Unit = x match
  case b @ Wide(_) => val i: Int = b
def e(x: Tree): Unit = x match
  case b @ Bounded(c) => val i: Int = c
def f(x: Any): Unit = x match
  case b @ Heads(h) => val i: Int = h
def g[T](x: T): Unit = x match
  case b @ Narrow(_) => val i: Int = b
abstract class Holder:
  type U
  def h(x: U): Unit = x match
    case b @ Narrow(_) => val i: Int = b
trait A; trait B
object TakesB { def unapply(b: B): Option[Int] = Some(1) }
def k(x: A): Unit = x match
  case b @ TakesB(_) => val i: Int = b
