// A call runs the first definition with a body in the receiver's linearisation: a default that
// every instantiated class overrides is dropped, one that a class inherits stays, and every
// definition of a name that `super` calls stays. A class that declares a trait's member abstract
// again is the member's owner for a call through it, which an unrelated class of the trait does
// not answer.
abstract class Shape:
  def area: Double
  def name: String = "shape"
  def describe: String = name + " " + area

class Square(side: Double) extends Shape:
  def area: Double = side * side
  override def name: String = "square"

class Circle(r: Double) extends Shape:
  def area: Double = 3.0 * r * r
  override def name: String = "circle"

trait Greeter:
  def greet: String = "hello"
  def loud: String = greet.toUpperCase

class Plain extends Greeter
class Formal extends Greeter:
  override def greet: String = "good day"

trait Logged extends Greeter:
  override def greet: String = "[" + super.greet + "]"
class Noted extends Formal with Logged

trait Sized:
  def size: Int = "sized default".length
abstract class Bag extends Sized:
  override def size: Int
class Pair extends Bag:
  override def size: Int = 2
class Loose extends Sized:
  override def toString: String = "loose"

@main def run(): Unit =
  val shapes: List[Shape] = List(Square(2), Circle(1))
  shapes.foreach(s => println(s.describe))
  val gs: List[Greeter] = List(Plain(), Formal(), Noted())
  gs.foreach(g => println(g.loud))
  val bags: List[Bag] = List(Pair())
  bags.foreach(b => println(b.size))
  println(Loose())
