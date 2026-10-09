// A lambda whose body is a match, given to `collect` of a sequence and of an option, is a
// pattern-matching anonymous function: its cases, a selection on the parameter as the scrutinee
// and a guard among them, decide what is kept, and the match is not checked for exhaustivity
// (dotc's `ExpandSAMs`).
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
case class Item(id: String, shape: Shape)

object Main:
  def main(args: Array[String]): Unit =
    val items = List(Item("a", Circle(3)), Item("b", Square(2)), Item("c", Circle(9)))
    println(items.collect(it => it.shape match
      case Circle(r) => r))
    println(items.toVector.collect(it => it.shape match
      case Circle(r) if r > 5 => it.id))
    println(items.map(it => (it.id, it)).toSeq.collect(kv => kv._2.shape match
      case Square(s) => BigDecimal(s)))
    println(items.iterator.collect(it => it.shape match { case Square(s) => s }).toList)
    println(Option(Item("z", Square(1))).collect(it => it.shape match
      case Circle(r) => r))
    println(Option(Item("y", Circle(4))).collect(it => it.shape match
      case Circle(r) => r))
    println(items.collectFirst(it => it.shape match { case Square(s) => s }))
    println(List(1, 2, 3).collect(x => x match { case 2 => "two" }))
