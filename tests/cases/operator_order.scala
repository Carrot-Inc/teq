//> using platform js
import scala.collection.mutable
def t[A](l: String, a: A): A = { println("  " + l); a }
class Box(var v: Int)
case class Cell(var n: Int)
@main def run(): Unit =
  println((1 +: Array(2, 3)).toList)
  println(((1, "a")) :: List((2, "b")))
  println(List(1, 2).foldLeft(List.empty[Int])((acc, x) => t("f" + x, x * 2) :: acc))
  val counts = mutable.Map[String, Int]()
  for w <- List("a", "b", "a") do counts(w) = counts.getOrElse(w, 0) + 1
  counts("a") += 10
  println(counts.toList.sorted)
  val nested = mutable.Map("k" -> mutable.ArrayBuffer(1))
  nested("k") += 2
  println(nested)
  var xs = List(1)
  xs ::= 0
  xs :+= 2
  println(xs)
  val bs = Array(Box(1), Box(2))
  bs(t("i", 1)).v += t("r", 5)
  println(bs.map(_.v).toList)
  val cells = List(Cell(1))
  cells.head.n *= 3
  println(cells)
  var s = "a"
  s += t("b", "b")
  println(s)
  var d = 1.5
  d *= 2
  println(d)
  val arr2 = Array.fill(2)(0)
  var i = 0
  def next() = { i += 1; i - 1 }
  arr2(next()) += 7
  arr2(next()) += 8
  println(arr2.toList)
  println(t("h", 1) :: t("t", Nil))
  def f(a: Int, b: Int = t("db", 2), c: Int = 3) = a + b + c
  println(f(c = t("c", 30), a = t("a", 100)))
  println(f(1, c = 3, b = 2))
