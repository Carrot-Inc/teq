// The left operand of `&&` typed as any qualifier, under the selection of the operator (`typedInfixOp`,
// `shallowSelectionProto`): the overloaded `jittered` is resolved by the alternatives whose normalized type
// has a member `&&`, the implicit-only one (`resolveOverloaded1`, `normalize`), not against `Boolean`.
trait Trace
object Trace { given Trace = new Trace {} }
trait Schedule:
  def jittered(implicit trace: Trace): Schedule = this
  def jittered(min: Double, max: Double)(implicit trace: Trace): Schedule = this
  def &&(that: Schedule)(implicit trace: Trace): Schedule = that
object Schedule:
  def exponential(n: Int): Schedule = new Schedule {}
  def recurs(n: Int): Schedule = new Schedule { override def toString = s"recurs($n)" }
@main def run(): Unit =
  val r = Schedule.exponential(2).jittered && Schedule.recurs(6)
  println(r)
