// `infix_overload_implicit_and` with `++`, which master already resolves.
trait Trace
object Trace { given Trace = new Trace {} }
trait Schedule:
  def jittered(implicit trace: Trace): Schedule = this
  def jittered(min: Double, max: Double)(implicit trace: Trace): Schedule = this
  def ++(that: Schedule)(implicit trace: Trace): Schedule = that
object Schedule:
  def exponential(n: Int): Schedule = new Schedule {}
  def recurs(n: Int): Schedule = new Schedule { override def toString = s"recurs($n)" }
@main def run(): Unit =
  val r = Schedule.exponential(2).jittered ++ Schedule.recurs(6)
  println(r)
