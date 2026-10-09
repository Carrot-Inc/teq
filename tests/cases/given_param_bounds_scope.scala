// A method's type parameter's implicit scope is the scope of its bounds, as an abstract type member's lower bound is in its scope
//> using scala 3.8.4
trait TC[A] { def s: String }
class Animal
object Animal { given [X <: Animal]: TC[X] = new TC[X] { def s = "animal-upper" } }
class Dog extends Animal
object Dog { given [X >: Dog]: TC[X] = new TC[X] { def s = "dog-lower" } }
object Main:
  def upper[T <: Animal]: String = summon[TC[T]].s
  def lower[T >: Dog]: String = summon[TC[T]].s
  def main(args: Array[String]): Unit =
    println(upper[Dog])
    println(lower[Animal])
