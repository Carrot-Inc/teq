//> using platform js
// A plain inline call whose `Int` the typing widens to `Double` in place (an argument, an
// ascribed val, a list's element): the later expansion keeps the widened type on the call's node,
// so the value is a `Double` on every target.
object M:
  inline def one: Int = 1

def show(d: Double): Unit = println(d)

@main def run(): Unit =
  show(M.one)
  val d: Double = M.one
  println(d)
  val xs: List[Double] = List(M.one, 2)
  println(xs)
