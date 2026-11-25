// `super.equals(a)` in a value class's method is Object's equals of a box nothing else holds (scalac boxes the value
// and calls a super accessor on the box): `false` for every argument, in the method's body, a lambda and a local
// def alike, the argument evaluated first.
class V(val x: Int) extends AnyVal:
  def withNull: Boolean = super.equals(null)
  def withOther(o: V): Boolean = super.equals(o)
  def withThis: Boolean = super.equals(this)
  def fromLambda: Boolean = { val f = () => super.equals(this); f() }
  def fromLocalDef: Boolean = { def g(): Boolean = super.equals(this); g() }
  def evaluated: Boolean = super.equals({ println("argument"); this })

object Main:
  def main(args: Array[String]): Unit =
    val v = new V(1)
    println(v.withNull)
    println(v.withOther(new V(1)))
    println(v.withThis)
    println(v.fromLambda)
    println(v.fromLocalDef)
    println(v.evaluated)
