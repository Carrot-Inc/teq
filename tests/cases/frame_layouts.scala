// The frames the interpreter opens and the places of the locals in them, each body called more
// than once so that its unit is promoted (src/interp/code.rs): blocks, lambdas and local defs
// over their enclosing frames, binding cases and alternatives, an operand pattern of a local read
// from two frames, defaults run under two layouts, catch cases, lazy locals, a local class made in
// its own method, a case lambda over a tuple and pattern definitions.
object Pos:
  def unapply(x: Int): Option[Int] = if x > 0 then Some(x) else None

class Calc(base: Int):
  def m(a: Int)(b: Int = a * 2, c: Int = base): Int = a + b * 10 + c * 100
  def twice(a: Int)(b: Int = a + base): Int = a * b

class Sub(base: Int) extends Calc(base):
  override def m(a: Int)(b: Int = a * 3, c: Int = base + 1): Int = super.m(a)(b, c) + 1000

object Main:
  def blocks(n: Int): Int =
    val a = n + 1
    val b = a * 2
    def later(): Int = a + b + n
    val f = (x: Int) => (y: Int) => x + y + a + b + n
    val g = { val c = a + b; (z: Int) => z * c }
    later() + f(1)(2) + g(3)

  def cases(x: Int, k: Int): String =
    val outer = x * 100
    x match
      case `k` => s"k $outer"
      case Pos(1) | Pos(2) => s"small $outer"
      case Pos(n) if n > outer / 200 => s"pos $n $outer"
      case n => s"other $n ${n + outer}"

  def defaults(c: Calc): Int = c.m(1)() + c.m(1)(2) + c.twice(3)() + c.twice(3)(4)

  def caught(i: Int): String =
    val label = "t" + i
    try
      if i % 2 == 0 then throw new IllegalStateException(label) else label.length.toString
    catch
      case e: IllegalStateException if e.getMessage == label => s"caught ${e.getMessage} $i"
    finally
      print("")

  def lazies(n: Int): Int =
    var count = 0
    lazy val x = { count += 1; n * 2 + count }
    val h = () => x + count
    val r = (if n > 1 then x else 0) + h()
    r + count

  def classes(n: Int): Int =
    val k = n * 3
    class Acc(val v: Int):
      def next: Acc = new Acc(v + k)
      def get: Int = v + k + n
    new Acc(1).next.next.get

  def tuples(n: Int): List[Int] =
    val outer = n
    List((1, 2), (3, 4), (5, 6)).map { case (a, b) => a * b + outer }

  def patterns(n: Int): Int =
    val (a, b) = (n, n * 2)
    val List(c, d) = List(a + b, a - b): @unchecked
    val Some(e) = Option(c * d): @unchecked
    a + b + c + d + e

  def loop(n: Int, fs: List[() => Int]): List[() => Int] =
    if n == 0 then fs else loop(n - 1, (() => n * 10) :: fs)

  def main(args: Array[String]): Unit =
    for i <- 1 to 3 do
      println(blocks(i))
      println(cases(i, 3))
      println(cases(i + 4, 3))
      println(defaults(new Calc(i)))
      println(defaults(new Sub(i)))
      println(caught(i))
      println(lazies(i))
      println(classes(i))
      println(tuples(i).mkString(","))
      println(patterns(i))
      println(loop(i, Nil).map(_()).mkString(","))
