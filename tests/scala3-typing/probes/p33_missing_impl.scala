trait Animal:
  def sound: String
  def legs: Int
class Cat extends Animal:
  def sound = "meow"
object Dog extends Animal:
  def sound = "woof"
  def legs = 4
trait Named:
  val name: String
class Anon extends Named
@main def run(): Unit = println(Cat().sound)
