// A lambda whose body is a match, given to Map's overloaded `collect` (both alternatives take a
// partial function), is a pattern-matching anonymous function: its cases decide `isDefinedAt`,
// whatever the scrutinee (a selection on the parameter), and the match is not checked for
// exhaustivity (dotc's `ExpandSAMs`). A tuple result picks the alternative that builds a map;
// the scrutinee is evaluated once per element, by `applyOrElse`.
import scala.collection.immutable.{SortedMap, TreeMap}

sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
case class Item(id: String, shape: Shape)

object Main:
  var reads = 0
  def shapeOf(i: Item): Shape = { reads += 1; i.shape }

  def main(args: Array[String]): Unit =
    val m = Map("a" -> Item("a", Circle(3)), "b" -> Item("b", Square(2)), "c" -> Item("c", Circle(5)))
    println(m.collect(it => it._2.shape match
      case Circle(r) => r))
    println(m.collect(it => it._2.shape match
      case Circle(r) => (it._1, r)))
    println(m.collect { it => it._2.shape match { case Square(s) => s } })
    println(m.collect(it => { it._2.shape match { case Square(s) => s } }))
    println(m.collect(_._2.shape match { case Circle(r) => r * 10 }))
    println(m.collect(it => shapeOf(it._2) match { case Circle(r) => r }))
    println(reads)
    val sorted: SortedMap[Int, Shape] = TreeMap(1 -> Circle(1), 2 -> Square(2), 3 -> Circle(3))
    println(sorted.collect(kv => kv._2 match { case Circle(r) => (kv._1, r) }))
    println(sorted.collect(kv => kv._2 match { case Circle(r) => r }))
    val empty = m.collect(it => it._2.shape match { case Circle(r) if r > 100 => r })
    println(empty.isEmpty)
