// A method whose using clauses are left, expanded where a function returning a context function
// is expected, takes them from that function's parameters: `(x) => (c) ?=> m(x)(using c)`.
class Ctx(val name: String)
class Me(val id: Int)

object Report:
  def onAction(goTo: String)(using ctx: Ctx): String = s"$goTo ${ctx.name}"
  def onModAction(goTo: String)(using ctx: Ctx, me: Me): String = s"$goTo ${ctx.name} ${me.id}"

def run(thenWhat: String => Ctx ?=> String): String =
  given Ctx = Ctx("c")
  thenWhat("x")

def runA[A](a: A)(thenWhat: A => Ctx ?=> String): String =
  given Ctx = Ctx("d")
  thenWhat(a)

def body[A](f: Me ?=> Option[A])(thenWhat: A => (Ctx, Me) ?=> String): String =
  given Ctx = Ctx("e")
  given Me = Me(7)
  f.fold("none")(a => thenWhat(a))

@main def main(): Unit =
  // The closure's parameters, nearer than these, are the ones found.
  given Ctx = Ctx("outer")
  given Me = Me(0)
  println(run(Report.onAction))
  println(runA("y")(Report.onAction))
  println(body(me ?=> Some(s"user${me.id}"))(Report.onModAction))
  val plain: String => String = Report.onAction
  println(plain("z"))
