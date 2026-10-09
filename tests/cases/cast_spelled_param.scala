// A parameter whose type is a `*:` chain ending in the alias `EmptyTuple` erases to `Product`, and
// its body reads it as the tuple class, through a cast checked at each read (scalac's typer casts
// the chain to its tuple class where it selects from it, `Typer.trySmallGenericTuple`): any product
// reaches the method, a pair passes the read, another product fails it.
case class P(n: Int, s: String)
object Cons:
  def first(t: Int *: String *: EmptyTuple): Int = t._1
  def both(t: Int *: String *: EmptyTuple): Int = t._1 + t._2.length
  def passed(t: Int *: String *: EmptyTuple): Any = t
def attempt(label: String)(f: => Any): Unit =
  try println(label + " " + f)
  catch case _: ClassCastException => println(label + " CCE")
@main def run(): Unit =
  attempt("pair") { Cons.both((1, "ab")) }
  val p: Any = P(2, "cd")
  attempt("product") { Cons.first(p.asInstanceOf[Int *: String *: EmptyTuple]) }
  attempt("product passed on") { Cons.passed(p.asInstanceOf[Int *: String *: EmptyTuple]) }
  attempt("string") { Cons.first(("x": Any).asInstanceOf[Int *: String *: EmptyTuple]) }
