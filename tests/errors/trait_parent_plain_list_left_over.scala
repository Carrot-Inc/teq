// A non-empty plain list meeting a trait's normalized empty clause (before a leading implicit clause, after using
// clauses alone): too many arguments, at the first of them, as scalac refuses it; one meeting the implicit clause
// after that empty clause is typed as written, with scalac's migration warning. By a parent and an anonymous class.
// expect: 26:23: warning: Implicit parameters should be provided with a `using` clause.
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect+ To disable the warning, please use the following option:
// expect+   "-Wconf:msg=Implicit parameters should be provided with a `using` clause:s"
// expect: 27:21: error: too many arguments for constructor Ti in trait Ti: ()(implicit a: A, b: B): Ti
// expect: 28:21: error: too many arguments for constructor Tu in trait Tu: (): Tu
// expect: 29:45: error: too many arguments for constructor Tu in trait Tu: (): Tu
// expect: 31:21: warning: Implicit parameters should be provided with a `using` clause.
// expect+ This code can be rewritten automatically under -rewrite -source 3.7-migration.
// expect+ To disable the warning, please use the following option:
// expect+   "-Wconf:msg=Implicit parameters should be provided with a `using` clause:s"
// expect: 32:19: error: too many arguments for constructor Ti in trait Ti: ()(implicit a: A, b: B): Ti
// expect: 33:19: error: too many arguments for constructor Tu in trait Tu: (): Tu
// expect: 34:43: error: too many arguments for constructor Tu in trait Tu: (): Tu
class A(val s: String)
class B(val s: String)
given A = A("ga")
given B = B("gb")
trait Ti(implicit a: A, b: B):
  def show: String = a.s + " " + b.s
trait Tu(using a: A, b: B):
  def show: String = a.s + " " + b.s
class P1 extends Ti()(A("wa"), B("wb"))
class P3 extends Ti(A("wa"), B("wb"))
class P2 extends Tu(A("wa"), B("wb"))
class P4 extends Tu(using A("wa"), B("wb"))(A("x"))
object O:
  val q1 = new Ti()(A("wa"), B("wb")) {}
  val q3 = new Ti(A("wa"), B("wb")) {}
  val q2 = new Tu(A("wa"), B("wb")) {}
  val q4 = new Tu(using A("wa"), B("wb"))(A("x")) {}
