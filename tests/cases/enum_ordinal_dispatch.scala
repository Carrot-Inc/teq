// `ordinal` called through `scala.reflect.Enum` is the receiver's: dotty's `Enum.ordinal` is an
// ordinary abstract member, a class extending `Enum` implements it as it likes (a def, a val), and
// each enum case has the implementation `DesugarEnums` gives it (a value case's, a class case's, a
// parameterised enum's), also behind a generic receiver and in a list of them.
case class Manual(x: Int) extends scala.reflect.Enum:
  def ordinal: Int = 42

case class ByVal(x: Int) extends scala.reflect.Enum:
  val ordinal: Int = 7

class Counted extends scala.reflect.Enum:
  var calls = 0
  def ordinal: Int =
    calls += 1
    calls * 10
  def productArity: Int = 0
  def productElement(n: Int): Any = throw new IndexOutOfBoundsException(n.toString)
  def canEqual(that: Any): Boolean = false

enum Color:
  case Red, Green
  case Mix(n: Int)

enum Planet(val mass: Double):
  case Mercury extends Planet(3.3e23)
  case Earth extends Planet(5.97e24)

object Main:
  def ord(e: scala.reflect.Enum): Int = e.ordinal
  def ordOf[E <: scala.reflect.Enum](e: E): Int = e.ordinal

  def main(args: Array[String]): Unit =
    val e: scala.reflect.Enum = Manual(1)
    println(e.ordinal)
    println(ord(ByVal(1)))
    println(ord(Color.Mix(3)))
    println(ord(Color.Green))
    println(ordOf(Planet.Earth))
    val c = new Counted
    println(ord(c) + ord(c))
    println(c.calls)
    val all: List[scala.reflect.Enum] = List(Color.Red, Manual(2), Color.Mix(1), Planet.Mercury)
    println(all.map(_.ordinal))
