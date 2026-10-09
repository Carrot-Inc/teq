// Case classes defined in a block: lifted with what they capture, with the members of a case
// class (apply, copy, unapply, equality, toString) and a companion object in the same block.
trait Labelled:
  def label: String

object Main:
  def points(scale: Int): String =
    case class Point(x: Int, y: Int) extends Labelled:
      def label: String = s"P($x,$y)x$scale"
      def scaled: Point = copy(x = x * scale, y = y * scale)
    val p = Point(1, 2)
    val Point(a, b) = p.scaled
    val same = p == Point(1, 2) && p.hashCode == Point(1, 2).hashCode
    val shown = List[Any](p, "s", p.scaled).collect { case q: Point => q.label }.mkString(" ")
    s"$p ${p.scaled} $a $b $same $shown ${p.copy(y = 9)} ${p.productPrefix}"

  def withCompanion(base: Int): String =
    case class Amount(value: Int, unit: String = "kg")
    object Amount:
      def zero: Amount = Amount(0, "g")
      def parse(s: String): Option[Amount] = s.toIntOption.map(v => Amount(v + base))
    val parsed = Amount.parse("5")
    val text = parsed match
      case Some(Amount(v, u)) => s"$v$u"
      case None => "none"
    s"${Amount.zero} $text ${Amount(3)} ${Amount.parse("x")}"

  def matchers(min: Int): String =
    case class Range(lo: Int, hi: Int):
      def contains(v: Int): Boolean = v >= lo && v <= hi
    val ranges = List(Range(0, min), Range(min + 1, min * 2))
    (0 to min * 2).map { v =>
      ranges.zipWithIndex.collectFirst { case (r, i) if r.contains(v) => i }.getOrElse(-1)
    }.mkString(",")

  def generic[T](items: List[T]): String =
    case class Wrapped(value: T, index: Int)
    val ws = items.zipWithIndex.map((v, i) => Wrapped(v, i))
    ws.map { case Wrapped(v, i) => s"$i:$v" }.mkString(" ")

  def counter(step: Int): () => String =
    var count = 0
    case class Tick(n: Int):
      def next: Tick =
        count += step
        Tick(n + 1)
    var t = Tick(0)
    () =>
      t = t.next
      s"${t.n}/$count"

  def main(args: Array[String]): Unit =
    println(points(3))
    println(points(10))
    println(withCompanion(100))
    println(matchers(2))
    println(generic(List("a", "b")))
    println(generic(List(1.5)))
    val tick = counter(5)
    tick()
    println(tick())
    val other = counter(1)
    println(other())
    val f = (m: Int) =>
      case class Pair(a: Int, b: Int = m)
      Pair(1).toString + Pair(1, 2).b
    println(f(9))
