// jars: scala-library
// std: lean scala-library
// Value classes of the program in link mode, erased as scalac erases them: the underlying type in
// descriptors and locals, the static `m$extension` beside the box's forwarding method, the box
// where a type parameter or `Any` takes the value (a `List`, an `Option`, a pattern on `Any`), and
// a default argument of scala-library's `StringOps.indexWhere` through `indexWhere$default$2$extension`.
class Meters(val v: Double) extends AnyVal:
  def plus(o: Meters): Meters = new Meters(v + o.v)
  override def toString = s"Meters($v)"
case class Name(s: String) extends AnyVal:
  def upper: Name = Name(s.toUpperCase)
class Box[A](val a: A) extends AnyVal:
  def get: A = a
object Main:
  def twice(m: Meters): Meters = m.plus(m)
  def main(args: Array[String]): Unit =
    val m = twice(new Meters(1.5))
    println(m)
    println(m.v)
    val xs = List(new Box(1), new Box(2))
    println(xs.map(_.get))
    val any: Any = m
    println(any)
    val n = Name("ann").upper
    println(n)
    println(List(n, Name("b")))
    println(n == Name("ANN"))
    any match
      case mm: Meters => println(mm.plus(mm))
      case _ => println("no")
    val o: Option[Meters] = Some(m)
    println(o.map(_.v))
    println((1 -> "a", "x" -> 2))
    println(3.max(4) + 10.min(2))
    println(" ab ".trim.capitalize)
    println("hello world".indexWhere(_ == 'o') + " " + "hello world".indexWhere(_ == 'o', 5))
