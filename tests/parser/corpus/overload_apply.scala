//> using scala 3.8.4
case class Money(cents: Long)
object Money:
  def apply(units: Int, cents: Int): Money = Money(units * 100L + cents)
  def apply(text: String): Money = Money(text.toLong)

class Order(val id: Int, val lines: List[String])
object Order:
  def apply(id: Int): Order = new Order(id, Nil)
  def apply(id: Int, first: String): Order = new Order(id, List(first))

class Registry:
  private var items = List.empty[String]
  def apply(i: Int): String = items(i)
  def apply(name: String): Int = items.indexOf(name)
  def update(i: Int, v: String): Unit = items = items.updated(i, v)
  def add(s: String): Registry = { items = items :+ s; this }

trait Show[A]:
  def show(a: A): String
given Show[Int] with
  def show(a: Int) = "int:" + a
given Show[String] with
  def show(a: String) = "str:" + a

object Printer:
  def print[A](a: A)(using s: Show[A]): String = s.show(a)
  def print[A](a: A, b: A)(using s: Show[A]): String = s.show(a) + "," + s.show(b)
  def pair[A, B](a: A, b: B): String = "pair"
  def pair[A](a: A, b: A, c: A): String = "triple"
  def first[A](xs: List[A]): A = xs.head
  def first[A](xs: Option[A]): A = xs.get
  def max(a: Int, b: Int): Int = if a > b then a else b
  def max(a: Long, b: Long): Long = if a > b then a else b
  def sum(xs: Int*): Int = xs.sum
  def sum(label: String, xs: Int*): String = label + xs.sum

@main def run(): Unit =
  println(Money(12, 34))
  println(Money("99"))
  println(Money(5L))
  println(Order(1).lines)
  println(Order(2, "pen").lines)
  val r = Registry().add("a").add("b")
  println(r(1))
  println(r("b"))
  r(0) = "z"
  println(r(0))
  println(Printer.print(1))
  println(Printer.print("a", "b"))
  println(Printer.pair(1, "x"))
  println(Printer.pair(1, 2, 3))
  println(Printer.first(List(1, 2)))
  println(Printer.first(Some("x")))
  println(Printer.max(1, 2))
  println(Printer.max(1L, 2))
  println(Printer.sum(1, 2, 3))
  println(Printer.sum("total ", 1, 2))
  println(Printer.sum())
  val fs: List[Int => Order] = List(Order.apply)
  println(fs.head(7).id)
