// From scala/scala3 tests/run/implicitclasses.scala and tests/pos/implicitonSelect.scala
// (`extends App` replaced by a main method, the assertion printed).
import scala.language.implicitConversions
object Test {
  implicit class C(s: String) {
    def nElems = s.length
  }

  class A
  class B { override def toString = "B" }
  implicit def a2b(x: A): B = new B
  class ARef { val a: A = new A }
  val x = new ARef
  val b: B = x.a

  def main(args: Array[String]): Unit = {
    println("abc".nElems)
    println(b)
  }
}
