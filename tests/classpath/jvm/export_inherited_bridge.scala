// jars: scala-library
// std: scala-library
// An inherited export implementing a generic abstract member: the class mixing the trait in gets the erased
// bridge, as scalac's.
object Impl { def f(x: String): String = x + "!" }
trait Parent { export Impl.f }
trait API[A] { def f(x: A): A }
class Child extends Parent with API[String]
class Car extends API[String] { export Impl.f }
class SubCar extends Car
object Main:
  def main(args: Array[String]): Unit =
    val a: API[String] = new Child
    println(a.f("x"))
    val b: API[String] = new SubCar
    println(b.f("y"))
