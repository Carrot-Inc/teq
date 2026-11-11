package creatorCalls
case class Defaulted(name: String, age: Int = 4)
case class Repeated(values: Int*)
case class Contextual(value: Int)(using val label: String)
case class Curried(first: Int)(val second: Int)
case class Delayed(first: Int)(second: => Int):
  def total: Int = first + second
case class Values[A](elements: A*)
case class EmptyFirst()(val n: Int)
class Enclosing(val seed: Int):
  case class Inner(value: Int = seed)
object Calls:
  given String = "context"
  def defaulted = Defaulted("person")
  def repeated = Repeated(1, 2)
  def contextual = Contextual(3)
  def curried = Curried(4)(5)
  def lifted = Defaulted(age = 6, name = "named")
  def delayed = Delayed(2)(3)
  def values = Values[String]("a", "b")
  def emptyFirst = EmptyFirst()(8)
  val scope = new Enclosing(9)
  def nested = scope.Inner()
