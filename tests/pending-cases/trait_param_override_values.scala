// A trait's val parameter overridden by a class's (`cannot override val parameter value s in trait T`, a warning
// scalac 3.8.4 gives as teq does): scalac prints the class's value, `1 1`; teq's interpreter and JavaScript bind the
// trait's parameter over the class's (`$traitInit`, `trait_init`), `2 2`, master as well. Queued apart from
// enum-override-param, whose rule for a superclass's parameter (`overridden_by_param`) the trait's binding lacks.
trait T(val s: Int):
  def f = s
  val t = f
class C(override val s: Int) extends T(s + 1)

@main def run(): Unit =
  val c = C(1)
  println(s"${c.s} ${c.t}")
