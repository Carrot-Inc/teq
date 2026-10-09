package capture.calls

class G[T](val v: T):
  def id(x: T): T = x
  def poly[U](u: U): (T, U) = (v, u)

object Effects:
  def effect(s: String): Int = { print(s); s.length }
  def f(a: Int, b: Int): Int = a - b
  def d(a: Int = 1, b: Int = 2): Int = a + b
  def byN(x: => Int): Int = x
  def rep(xs: Int*): Int = xs.sum
  def curried(a: Int)(b: Int)(using s: String): Int = a + b + s.length

class Probe:
  import Effects.*
  def named: Int = f(b = effect("b"), a = effect("a"))
  def defaults: Int = d() + d(b = 5)
  def byname: Int = byN(effect("x"))
  def reps: Int = rep(1, 2) + rep(List(1)*) + rep()
  def cmp(i: Int, l: Long): Boolean = i < l
  def arith(i: Int, l: Long, d: Double): Double = i + l + d
  def concat(x: Any, y: Int): String = "a" + x + y + x.toString
  def concat2(y: Int): String = y + "a"
  def interp(x: Int): String = s"v=$x"
  def test(x: Any): Boolean = x.isInstanceOf[List[Int]]
  def generic: Int = new G[String]("s").id("t").length + new G(1).poly[String]("u")._1
  def forc(xs: List[Int]): List[Int] = for x <- xs; y = x + 1; if y > 2; (a, b) <- List((x, y)) yield a + b
  def localInline: Int =
    inline val k = 3
    k + 1
  def ret(xs: List[Int]): Int =
    xs.foreach(x => if x > 1 then return x)
    0
  def pat(x: Any): Int = x match
    case (a: Int, b: String) => a + b.length
    case l @ List(_, _*) => l.size
    case _ => 0
  def annot(x: Option[Int]): Int = (x: @unchecked) match
    case Some(v) => v
  def eta: List[Int] => List[Int] = _.map(identity)
  def etaM = f
  def curriedCall: Int =
    given String = "g"
    curried(1)(2)
  class Inner:
    def outer: Probe = Probe.this
    def viaOuter: Int = named
