package fix.shapes

// Completing these parents or bounds reaches the companion before the class
// has installed its constructor signature. Proxy entry must read symbols only.
trait CtorCycleParent[A]
class CtorCycleLinked(val n: Int) extends CtorCycleParent[CtorCycleLinked.Token.type] {
  def this() = this(0)
}
object CtorCycleLinked { object Token }
class CtorCycleBound[A <: CtorCycleBound.Bound](val value: A)
object CtorCycleBound {
  trait Bound
  object Value extends Bound
}
