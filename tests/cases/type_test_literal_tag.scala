// A pattern over an abstract type with a `TypeTest[1, T]` in scope calls the tag, over a scrutinee of the literal
// type `1` too, unless `T` is the scrutinee's type (dotty's `tryWithTypeTest`, its guards on type equality):
// a tag that answers `None` makes the case fail, and it is called once per test, a bound on `T` or not.
import scala.reflect.TypeTest

var calls = 0
def check[T](using TypeTest[1, T]): Boolean = (1: 1) match
  case _: T => true
  case _ => false
def bounded[T >: 1](using TypeTest[1, T]): Boolean = (1: 1) match
  case _: T => true
  case _ => false

@main def main(): Unit =
  given TypeTest[1, String] = new TypeTest[1, String]:
    def unapply(x: 1): Option[x.type & String] =
      calls += 1
      None
  println(check[String])
  println(calls)
  given TypeTest[1, Any] = new TypeTest[1, Any]:
    def unapply(x: 1): Option[x.type & Any] =
      calls += 1
      None
  println(bounded[Any])
  println(calls)
