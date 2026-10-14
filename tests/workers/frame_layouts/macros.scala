import scala.quoted.*

// A macro's code over the frames the interpreter opens (src/interp/code.rs), run by every
// expansion, so that its units are promoted on the workers as at one: blocks, lambdas and local
// defs over their enclosing frames, binding cases and an alternative, an operand pattern of a
// local, defaults under two layouts, a catch case, a lazy local, a local class made in its own
// method and a case lambda over a tuple.
object Pos:
  def unapply(x: Int): Option[Int] = if x > 0 then Some(x) else None

class Calc(base: Int):
  def m(a: Int)(b: Int = a * 2, c: Int = base): Int = a + b * 10 + c * 100

object M:
  def frames(n: Int, k: Int): String =
    val a = n + 1
    val b = a * 2
    def later(): Int = a + b + n
    val f = (x: Int) => (y: Int) => x + y + a + b
    val label = "t" + n
    lazy val l = a * 7
    class Acc(val v: Int):
      def next: Acc = new Acc(v + b)
      def get: Int = v + b + n
    val matched = n match
      case `k` => "k"
      case Pos(1) | Pos(2) => "small"
      case Pos(m) if m > a => "pos"
      case _ => "other"
    val caught =
      try
        if n % 2 == 0 then throw new IllegalStateException(label) else label
      catch case e: IllegalStateException if e.getMessage == label => "caught"
    val pairs = List((1, 2), (3, 4)).map { case (p, q) => p * q + n }
    val (u, v) = (a, b)
    List(later(), f(1)(2), new Calc(n).m(1)(), new Calc(n).m(1)(5), l, new Acc(1).next.get, u + v).mkString(",") + s" $matched $caught ${pairs.mkString(",")}"

  def impl(x: Expr[Int], k: Expr[Int])(using Quotes): Expr[String] =
    val n = x.valueOrAbort
    val three = k.valueOrAbort
    Expr(frames(n, three) + " " + frames(n + 1, three))

inline def show(inline x: Int): String = ${ M.impl('x, '{3}) }
