// After Scala 3's tests/pos/t175.scala (a secondary constructor of an abstract class) and
// tests/run/constructors.scala: secondary constructors on a class, a case class, a generic
// class, a subclass and a chain of them.
class Point(val x: Int, val y: Int):
  def this(x: Int) = this(x, 0)
  def this() = { this(7); println(s"made $x") }
  def this(s: String) = this(s.length, s.length * 2)
  override def toString = s"Point($x, $y)"

class Named(val name: String, val p: Point):
  println(s"named $name")
  def this(name: String) = { this(name, new Point(1)); println(s"one-arg $name") }
  def describe: String = s"$name at $p"

class Sub(n: String) extends Named(n):
  println("sub body")

class SubSub(n: String, val extra: Int) extends Sub(n):
  println(s"subsub $extra")

class Box[T](val v: T):
  def this(v: T, twice: Boolean) = this(if twice then v else v)
  def this() = this(null.asInstanceOf[T], true)

class Ratio private (val n: Int, val d: Int):
  def this(n: Int) = this(n, 1)
  override def toString = s"$n/$d"

case class Money(cents: Long, currency: String):
  def this(dollars: Int) = this(dollars * 100L, "USD")

object Money:
  def apply(s: String): Money = new Money(s.toInt)

class Defaults(val a: Int, val b: String):
  def this(a: Int, b: String, c: Boolean = true) = this(a, if c then b else b.reverse)

class Chained(val log: List[String]):
  def this(s: String) = { this(List(s)); println(s"chained $s") }
  def this(n: Int) = { this(n.toString); println(s"from int $n") }

class Wrapped(msg: String) extends Exception(msg):
  def this(code: Int) = this(s"code $code")

class Counter(var n: Int):
  def this(start: Int, step: Int) = { this(start); n += step }

@main def m(): Unit =
  println(new Point(1, 2))
  println(new Point(3))
  println(new Point())
  println(new Point("ab"))
  println(Point(4))
  val named = new Named("k")
  println(named.describe)
  println(new Sub("s").describe)
  val ss = new SubSub("t", 9)
  println(ss.extra)
  println(new Box[String]("x", false).v)
  println(new Box[Int]().v)
  println(new Ratio(3))
  println(new Money(5))
  println(Money("6"))
  println(Money(1L, "EUR"))
  println(new Defaults(1, "abc").b)
  println(new Defaults(1, "abc", false).b)
  println(new Chained(42).log)
  val w = new Wrapped(404)
  println(w.getMessage)
  try throw new Wrapped(500)
  catch case e: Exception => println(s"caught ${e.getMessage}")
  println(new Counter(10, 5).n)
  val cause = new IllegalStateException("inner")
  val e = new RuntimeException(cause)
  println(e.getMessage)
  println(e.getCause eq cause)
  println(new StringBuilder("ab").append("c").toString)
