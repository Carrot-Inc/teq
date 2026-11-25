package vcse

// A value class's `super.equals(..)`: scalac's `false` for every argument (Object's equals of a box nothing else holds).
class V(val x: Int) extends AnyVal:
  def withThis: Boolean = super.equals(this)
  def fromLambda: Boolean = { val f = () => super.equals(this); f() }
  def fromLocalDef: Boolean = { def g(): Boolean = super.equals(this); g() }
