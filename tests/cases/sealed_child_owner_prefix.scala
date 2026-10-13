// A sealed parent's child declared in an object of the parent's class (`O.T`'s `O.Nest.I`) is seen
// through the scrutinee's prefix and that object (`a.Nest.I`, `TypeOps.childPrefix`'s walk over the
// owners), so `Some(a.Nest.I(n))` and `None` cover an `Option[a.T]`, as `a.Nest.I(n)` covers an `a.T`.
class O:
  sealed trait T
  object Nest:
    case class I(n: Int) extends T
val a = new O
def f(x: Option[a.T]): Int = x match
  case Some(a.Nest.I(n)) => n
  case None => 0
def g(x: a.T): Int = x match
  case a.Nest.I(n) => n

@main def run(): Unit =
  println(f(Some(a.Nest.I(3))))
  println(f(None))
  println(g(a.Nest.I(4)))
