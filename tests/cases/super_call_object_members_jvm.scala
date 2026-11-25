//> using platform jvm
// `super.toString` over a case class (its synthesized toString stands behind the caller) and `super.clone()` over
// `Object`, as scalac writes them (`invokespecial` on the direct superclass); `super.hashCode()` and `super.equals(..)`
// in a value class's method, whose JVM method takes the underlying value. The JVM alone: JavaScript and the
// interpreter print another value for the first and stop at the second (master's own faults, their rows).
case class K(x: Int)
class OverCase extends K(1):
  override def toString(): String = "case:" + super.toString()
class Cl extends Cloneable:
  override def clone(): Object = super.clone()
class Vc(val x: Int) extends AnyVal:
  def probe(): Boolean =
    super.hashCode()
    super.equals(null)

object Main:
  def main(args: Array[String]): Unit =
    println(new OverCase().toString)
    println(new Cl().clone().getClass.getSimpleName)
    println(new Vc(1).probe())
