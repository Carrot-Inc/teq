// jars: scala-library
// A value class that inherits hashCode, toString or equals from a universal trait has no synthesized one: the box
// takes the trait's (SyntheticMembers.existingDef), unless a trait after it declares the member abstract again;
// another signature of the name is no definition. A canEqual of the class's own is asked after the values compare.
trait H extends Any { override def hashCode: Int = 42; override def toString: String = "H!" }
trait E extends Any { override def equals(o: Any): Boolean = true }
class V(val x: Int) extends AnyVal with H
case class CV(x: Int) extends AnyVal with H with E
case class CE(x: Int) extends AnyVal { def canEqual(a: Any): Boolean = { Main.calls += 1; false } }
trait O extends Any { def canEqual(s: String): Boolean = false }
case class CO(x: Int) extends AnyVal with O
trait R extends Any with H { override def hashCode: Int }
class VR(val x: Int) extends AnyVal with R
object Main {
  var calls = 0
  def equal[A](a: A, b: A): Boolean = a == b
  def box[A](a: A): Any = a
  def main(args: Array[String]): Unit = {
    println(new V(1).hashCode); println(box(new V(1)).hashCode); println(box(new V(1))); println(new V(1).toString)
    println(CV(1).hashCode); println(box(CV(1)).hashCode); println(box(CV(1))); println(CV(1) == CV(2)); println(box(CV(1)) == box(CV(2)))
    println(CV(1).productPrefix)
    println(equal(CE(1), CE(1))); println(calls); println(equal(CE(1), CE(2))); println(calls)
    println(equal(CO(1), CO(1))); println(box(new VR(7)).hashCode)
  }
}
