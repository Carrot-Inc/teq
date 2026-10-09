// The expected type instantiates the type parameters of an application before its arguments
// are adapted: a literal widens or converts into the bound the expected type gave a type
// variable, an inner application keeps the variables of the enclosing one open, and a union
// selects members through its join.
class Money(val cents: Int):
  override def toString = s"Money($cents)"
object Money:
  implicit def intToMoney(i: Int): Money = new Money(i * 100)

case class Page[T](atCursor: Option[String], data: List[T])
case class Data[T](pages: List[Page[T]], next: Option[String])
case class Wrap[T](value: T)
sealed trait Remote[+A]
case class Loaded[A](value: A) extends Remote[A]
case object Empty extends Remote[Nothing]

object Main:
  def take(o: Option[Money]): String = o.toString
  def price(d: Option[Double]): Boolean = d.exists(_ > 9.5)

  def merge(remote: Remote[Data[String]], cursor: String): Remote[Data[String]] =
    val existing = (remote match { case Loaded(v) => Some(v); case Empty => None }).getOrElse(Data(List.empty, None))
    val result =
      if existing.pages.exists(_.atCursor.contains(cursor)) then existing
      else Data(existing.pages :+ Page(Some(cursor), List(cursor + "!")), None)
    Loaded(result)

  def pagesOf(flag: Boolean): Int =
    val d: Data[String] | Data[Int] = if flag then Data(List(Page(None, List("a"))), None) else Data(Nil, Some("n"))
    d.pages.size + d.next.size

  def main(args: Array[String]): Unit =
    val o: Option[Money] = Some(10)
    println(o)
    println(take(Some(3)))
    println(Wrap[Option[Money]](Some(4)))
    val ms: List[Money] = List(1, 2)
    println(ms)
    println(price(Some(10)))
    val d: Option[Double] = Some(3)
    println(d.map(_ + 0.5).exists(_ == 3.5))
    println(merge(Empty, "c"))
    println(merge(Loaded(Data(List(Page(Some("c"), List("x"))), None)), "c"))
    println(pagesOf(true) + pagesOf(false))
    val e: Either[String, Option[Money]] = Right(Some(7))
    println(e)
