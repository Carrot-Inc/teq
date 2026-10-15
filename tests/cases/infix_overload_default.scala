// Defaults do not make an ordinary clause implicit: `jittered(min = .., max = ..)` normalizes to a function
// type without `&&` (`ProtoTypes.normalize`).
trait Trace
object Trace { given Trace = new Trace {} }
trait Schedule:
  def jittered(implicit trace: Trace): Schedule = this
  def jittered(min: Double = 0.0, max: Double = 1.0)(implicit trace: Trace): Schedule = this
  def &&(that: Schedule)(implicit trace: Trace): Schedule = that
object Schedule:
  def exponential(n: Int): Schedule = new Schedule {}
  def recurs(n: Int): Schedule = new Schedule { override def toString = s"recurs($n)" }
@main def run(): Unit =
  val r = Schedule.exponential(2).jittered && Schedule.recurs(6)
  println(r)
