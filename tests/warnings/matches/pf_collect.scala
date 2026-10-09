// A lambda whose body is a match, given to `collect`: a pattern-matching anonymous function,
// whose match is not checked for exhaustivity, through Map's and SortedMap's overloads too.
import scala.collection.immutable.{SortedMap, TreeMap}

sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
case class Item(id: String, shape: Shape)

object Main:
  def main(args: Array[String]): Unit =
    val pair: (Int, Map[String, Item]) = (1, Map("a" -> Item("a", Circle(3)), "b" -> Item("b", Square(2))))
    val radii = pair._2.collect(it => it._2.shape match
      case Circle(r) => r)
    println(radii)
    println(pair._2.collect(it => it._2.shape match { case Circle(r) => (it._1, r) }))
    println(pair._2.collect { it => it._2.shape match { case Circle(r) => r } })
    println(pair._2.collect(it => { it._2.shape match { case Circle(r) => r } }))
    println(pair._2.collect(_._2.shape match { case Circle(r) => r }))
    println(pair._2.collect(it => (it._2.shape match { case Circle(r) => r })))
    val sorted: SortedMap[Int, Shape] = TreeMap(1 -> Circle(1), 2 -> Square(2))
    println(sorted.collect(kv => kv._2 match { case Circle(r) => (kv._1, r) }))
    println(sorted.collect(kv => kv._2 match { case Circle(r) => r }))
    val seq: Seq[(String, Item)] = pair._2.toSeq
    println(seq.collect(it => it._2.shape match
      case Circle(r) => BigDecimal(r)))
    println(Option(Item("z", Square(1))).collect(it => it.shape match
      case Circle(r) => r))
    println(seq.iterator.collect(it => it._2.shape match { case Square(s) => s }).toList)
    println(seq.collectFirst(it => it._2.shape match { case Square(s) => s }))
