// expect: type mismatch: found Box[String], required Box[? >: Dog]
// expect: type mismatch: found String, required
// expect: type mismatch: found Dog, required
trait Animal:
  def name: String
final case class Dog(name: String) extends Animal
final class Box[A](var value: A)

object Main:
  def first(b: Box[? <: Animal]): String = b.value.name
  def reset(b: Box[? >: Dog]): Unit = b.value = Dog("rex")
  def main(args: Array[String]): Unit =
    println(first(Box("str")))
    reset(Box[Animal](Dog("a")))
    val b: Box[? <: Animal] = Box(Dog("w"))
    b.value = Dog("x")
    reset(Box[String]("s"))
