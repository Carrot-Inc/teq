// The compile-time patterns of scala.compiletime over explicit tuple types: instances
// collected with summonInline over `h *: t`, dispatch on erasedValue, labels from constant
// types, and a type class derived for products described as tuples, without a Mirror.
import scala.compiletime.{constValue, constValueTuple, erasedValue, summonAll, summonFrom, summonInline}

trait Show[A]:
  def show(a: A): String

object Show:
  given Show[Int] with
    def show(a: Int): String = a.toString
  given Show[String] with
    def show(a: String): String = "\"" + a + "\""
  given Show[Boolean] with
    def show(a: Boolean): String = if a then "yes" else "no"

object Instances:
  inline def showsOf[T <: Tuple]: List[Show[?]] = inline erasedValue[T] match
    case _: EmptyTuple => Nil
    case _: (t *: ts) => summonInline[Show[t]] :: showsOf[ts]

  inline def sizeOf[T <: Tuple]: Int = inline erasedValue[T] match
    case _: EmptyTuple => 0
    case _: (_ *: ts) => 1 + sizeOf[ts]

  inline def encode[Fields <: Tuple](values: List[Any]): List[String] = inline erasedValue[Fields] match
    case _: EmptyTuple => Nil
    case _: (t *: ts) => summonInline[Show[t]].show(values.head.asInstanceOf[t]) :: encode[ts](values.tail)

  inline def labels[L <: Tuple]: List[String] = inline erasedValue[L] match
    case _: EmptyTuple => Nil
    case _: (l *: ls) => constValue[l].toString :: labels[ls]

  transparent inline def defaultOf[T] = inline erasedValue[T] match
    case _: Int => 0
    case _: String => ""
    case _: Boolean => false

  inline def orderingOr[T](inline fallback: String): String = summonFrom {
    case given Ordering[T] => "ordered"
    case _ => fallback
  }

case class Person(name: String, age: Int, active: Boolean)

object Person:
  inline def fieldLabels = labelsOf[("name", "age", "active")]
  private inline def labelsOf[L <: Tuple]: List[String] = Instances.labels[L]
  given Show[Person] with
    def show(p: Person): String =
      val values = Instances.encode[(String, Int, Boolean)](List(p.name, p.age, p.active))
      fieldLabels.zip(values).map((l, v) => l + "=" + v).mkString("Person(", ", ", ")")

object Main:
  def main(args: Array[String]): Unit =
    import Instances.*
    println(showsOf[(Int, String)].size)
    println(sizeOf[(Int, String, Boolean)] + sizeOf[EmptyTuple] + sizeOf[Tuple1[Int]])
    println(encode[(Int, String, Boolean)](List(1, "two", true)))
    println(labels[("x", "y")])
    println(defaultOf[Int].toString + "|" + defaultOf[String] + "|" + defaultOf[Boolean])
    println(orderingOr[Int]("none") + " " + orderingOr[Person]("none"))
    println(summon[Show[Person]].show(Person("ann", 33, false)))
    val (a, b) = constValueTuple[(7, "seven")]
    println(a + 1)
    println(b.length)
    val shows: (Show[Int], Show[Boolean]) = summonAll[(Show[Int], Show[Boolean])]
    println(shows._1.show(5) + shows._2.show(true))
