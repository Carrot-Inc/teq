// Two forms of a dynamic call that scalac builds in its own way: an infix operation's parenthesised list is one
// tuple to the dynamic method (the dotted call passes two arguments), and a written `using` list is passed as the
// arguments, the given in scope answering the method's own using clause.
import scala.language.dynamics

class Ctx(val n: Int)

class Plain extends Dynamic:
  def applyDynamic(name: String)(args: Any*): String = name + args.mkString("(", ",", ")")

class Scoped extends Dynamic:
  def selectDynamic(name: String)(using c: Ctx): String = s"select($name) ctx ${c.n}"
  def applyDynamic(name: String)(args: Any*)(using c: Ctx): String = s"apply($name) ${args.length} ctx ${c.n}"

@main def run(): Unit =
  val d = new Plain
  println(d call (1, 2))
  println(d.call(1, 2))
  println(d call ((1, 2)))
  given Ctx = new Ctx(7)
  val s = new Scoped
  println(s.a(using new Ctx(8)))
