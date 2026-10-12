// A context function literal whose parameters have no type takes the shape of a context function
// of its arity in overload resolution (dotty's typeShape, isContextual): the alternative whose
// parameter is no context function is ruled out, the other types it.
class Ctx(val n: Int)
class Me(val name: String)
class Parser[A](val label: String)

object C:
  def Auth(f: Ctx ?=> Me ?=> String): String = Auth(Parser[Unit]("empty"))(f)
  def Auth[A](p: Parser[A])(f: Ctx ?=> Me ?=> String): String =
    p.label + ": " + f(using Ctx(1))(using Me("me"))
  def a = Auth { _ ?=> me ?=> me.name }
  def b = Auth { ctx ?=> me ?=> s"${ctx.n} ${me.name}" }
  def c = Auth(Parser[Int]("ints")) { ctx ?=> _ ?=> ctx.n.toString }

@main def run(): Unit =
  println(C.a)
  println(C.b)
  println(C.c)
