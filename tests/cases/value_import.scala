// An import from a stable value (SLS 4.7) in a block: the member by its name or an alias, and
// an implicit member as a given of the block; a method without parameters is read at each use.
trait Show[A] { def show: String }
trait Ex:
  type U
  implicit val U: Show[U]
  val label: String
  def twice: String = label * 2
def use[A](implicit s: Show[A]): String = s.show
def f(e: Ex): String =
  import e.{U as Alias, label}
  use[e.U] + " " + label + " " + Alias.show
@main def main(): Unit =
  val ex = new Ex { type U = Int; val U = new Show[Int] { def show = "int" }; val label = "l" }
  println(f(ex))
  val g =
    import ex.label as name
    import ex.twice
    name * 2 + twice
  println(g)
