// A lambda's body of a context function type, typed against no result type or an open type
// variable, is applied to the givens in scope (dotty's adaptNoArgsOther inserts the `apply`).
class Ctx(val name: String)

def id[T](x: T): T = x
def fold[B](ifEmpty: => B)(f: Int => B): B = f(1)
def pick[B](f: Int => B): B = f(3)

@main def main(): Unit =
  given Ctx = Ctx("c")
  val g: Int => Ctx ?=> String = i => s"$i ${summon[Ctx].name}"
  println(id(g(1)))
  println(pick(a => g(a)))
  println(fold("none")(a => g(a)))
  println(Option(5).map(a => g(a)))
  println(Some(2).fold("none")(a => g(a)))
  val h = (a: Int) => g(a)
  val s: Int => String = h
  println(s(4))
  val k = (c: Ctx) ?=> g(6)
  val t: Ctx ?=> String = k
  println(t)
