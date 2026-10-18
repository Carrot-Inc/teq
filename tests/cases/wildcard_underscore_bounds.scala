// A wildcard type argument written `_` with bounds (`_ >: Integer`, `_ <: Number`), deprecated in Scala 3 and
// read by scalac 3.8.4 as `?` with the same bounds (dotty's `simpleType`: `typeBounds()` after `_`).
// teq: --wconf msg=is.deprecated.for.wildcard.arguments:s
import java.util.{ArrayList, List as JList}

def addOne(xs: JList[_ >: Integer]): Unit = xs.add(Integer.valueOf(1))
def total(xs: JList[_ <: Number]): Double =
  var sum = 0.0
  val it = xs.iterator()
  while it.hasNext do sum += it.next().doubleValue
  sum
def both(xs: Seq[_ >: Null <: AnyRef]): Int = xs.count(_ != null)

@main def main(): Unit =
  val numbers = new ArrayList[Number]()
  addOne(numbers)
  numbers.add(2.5)
  println(total(numbers))
  println(both(Seq("a", null, "b")))
