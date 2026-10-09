// expect: 14:30: error: type mismatch: found String, required Nothing
// expect: 17:20: error: type mismatch: found AnyRef, required String
// A generic `unapply` over a scrutinee that does not conform to its parameter maximises its
// type parameters as scalac's `maximizeType` does: down to the lower bound where the parameter
// is contravariant in it (the binder is a `Sink[Nothing]`, which takes no `String`), up to the
// upper bound otherwise, a lower bound notwithstanding (`v` is an `AnyRef`).
class Sink[-A]:
  def put(a: A): Int = 1
object Sinks { def unapply[T](s: Sink[T]): Some[Int] = Some(1) }
class Box[+A](val value: A)
object Boxes { def unapply[T >: String <: AnyRef](b: Box[T]): Some[T] = Some(b.value) }

def f(x: Any): Int = x match
  case b @ Sinks(n) => b.put("no")
  case _ => 0
def g(x: Any): String = x match
  case Boxes(v) => v
  case _ => ""
