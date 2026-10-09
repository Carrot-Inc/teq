// The cases of a pattern-matching anonymous function written as a lambda: a guard is part of
// `isDefinedAt`, a match nested in a case's body is not (the outer case decides, and the inner
// match throws `MatchError` when the function is applied), and a reference to the parameter in a
// guard or a body reads the argument (dotc's `ExpandSAMs`).
import scala.util.Try

sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
case class Item(id: String, shape: Shape)

object Main:
  def main(args: Array[String]): Unit =
    val guarded: PartialFunction[Item, Int] = it => it.shape match
      case Circle(r) if r > 2 && it.id != "skip" => r
    println(guarded.isDefinedAt(Item("a", Circle(1))))
    println(guarded.isDefinedAt(Item("a", Circle(3))))
    println(guarded.isDefinedAt(Item("skip", Circle(3))))
    println(guarded.isDefinedAt(Item("a", Square(3))))
    val nested: PartialFunction[Item, Int] = it => it.shape match
      case Circle(r) => r match
        case 1 => 10
    println(nested.isDefinedAt(Item("a", Circle(2))))
    println(nested.isDefinedAt(Item("a", Square(2))))
    println(nested(Item("a", Circle(1))))
    println(Try(nested(Item("a", Circle(2)))).failed.get.getClass.getSimpleName)
    val m = Map(1 -> Item("a", Circle(1)), 2 -> Item("b", Circle(2)), 3 -> Item("c", Square(3)))
    println(m.collect(kv => kv._2.shape match
      case Circle(r) if kv._1 > 1 => s"${kv._2.id}$r"))
    println(m.collect(kv => kv._2 match
      case Item(id, s) if id != "c" => s match
        case Circle(r) => r
        case Square(n) => -n))
