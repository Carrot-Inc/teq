// expect: 17:22: error: Found:    Box[String]⏎Required: Box[? >: Dog]
// expect: 13:23: error: Found:    "str"
// expect: 16:18: error: Found:    Dog
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
