// jars: scala-library vctraits-lib
// A value class inheriting a jar's universal trait's concrete hashCode, toString, equals and canEqual has no
// synthesized one: the box forwards to the trait's default, which the JVM would not pick over Object's.
import vct.*

class V(val u: Int) extends AnyVal with H
case class C(u: Int) extends AnyVal with H
case class E(u: Int) extends AnyVal with CE

object Main {
  def box[A](a: A): Any = a
  def equal[A](a: A, b: A): Boolean = a == b
  def main(args: Array[String]): Unit = {
    println(box(new V(1)).hashCode)
    println(box(new V(1)))
    println(equal(new V(1), new V(2)))
    println(box(C(1)).hashCode)
    println(box(C(1)))
    println(equal(C(1), C(2)))
    println(equal(E(1), E(1)))
  }
}
