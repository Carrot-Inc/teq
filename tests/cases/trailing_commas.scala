import scala.collection.mutable.{
  ArrayBuffer,
  HashMap,
}

case class Point[
  A,
  B,
](
  x: A,
  y: B,
)

def add(
  a: Int,
  b: Int,
): Int = a + b

def pick[
  T,
](
  xs: List[T],
)(
  using ord: Ordering[T],
): T = xs.max

def sizes(table: Map[
  String,
  Int,
]): List[Int] = table.values.toList

@main def main(): Unit =
  val pair = (
    1,
    "two",
  )
  println(pair)
  val xs = List(
    3,
    1,
    2,
  )
  println(xs)
  println(add(
    1,
    2,
  ))
  println(pick[
    Int,
  ](xs))
  val p = Point(
    x = 1,
    y = "y",
  )
  p match
    case Point(
          a,
          b,
        ) =>
      println(s"$a $b")
  val f = (
    a: Int,
    b: Int,
  ) => a * b
  println(f(3, 4))
  val buffer = ArrayBuffer(
    1,
  )
  buffer += 2
  println(buffer)
  val table = HashMap(
    "a" -> 1,
  )
  println(table)
  println(sizes(Map("k" -> 5,
  )))
  println(List(1, 2).map(n =>
    n + 1,
  ))
