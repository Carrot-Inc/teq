// An anonymous class over a class whose constructor parameter has a default, the default omitted.
class B(val x: List[Int] = List(1)):
  def first: Int = x.head
trait T
object Main:
  def main(args: Array[String]): Unit =
    val b = new B with T
    println(b.first)
