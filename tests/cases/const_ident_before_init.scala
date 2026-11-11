// A constant val read before its initialiser has run: FirstTransform takes the reference to its literal, so the read
// gives the constant, not the field's default; through a method of the object its body calls, and through a method a
// superclass's constructor calls.
object A:
  println(get)
  val x: 42 = 42
  def get: Int = x
abstract class Base:
  def value: Int
  println(value)
class B extends Base:
  val v: 7 = 7
  def value: Int = v
@main def main(): Unit =
  println(A.get)
  new B
  println("end")
