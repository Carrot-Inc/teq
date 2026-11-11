// A primitive unary operator or numeric conversion whose lowering hands on its operand's node (an Int's unary_+, a
// conversion to the receiver's own type) has the constant type ConstFold gives the operation (Typer.scala:805,
// ConstFold.scala:49-54), which an expected constant type reads, its operand an effectful selection or itself a unary
// operation; FirstTransform takes each to { pre; v } (TreeInfo.scala:753-786), so a read before the initialiser has
// run is the constant. From Codex's review of tr/cut-head-tail-c2 93e2011e3c (P1-1) and its check.
class S:
  def h: S = this
  val x: 42 = 42
  val l: 42L = 42L
class Holder:
  def h: Holder = { println("h"); this }
  val plus: Int = +(-h.x)
  val identity: Long = h.x.toLong.toLong
  val x: 42 = 42
object Expect:
  def d(c: S): 42 = c.h.x.toInt
  def d2(c: S): 42 = c.h.x.toInt.toInt
  def d4(c: S): 42L = c.h.l.toLong
  def r(c: S): 42 = { println("r"); c }.x.toInt
  def a1(c: S): 42 = { c.h.x.toInt }
  def a2(c: S): 42 = (c.h.x.toInt: 42)
  def a4(c: S): Int = { val v: 42 = (c.h.x.toInt: 42); v }
  def a9(c: S): Int = { val v: 42 = (c.h.x.unary_+); v }
  def i(c: S): 42 = if c eq null then 42 else c.h.x.toInt
  def take(v: 42): Int = v
  def t(c: S): Int = take(c.h.x.toInt)
  val f: S => 42 = c => c.h.x.toInt
  def p(c: S): -42 = +(-c.h.x)
  def w(c: S): 42L = c.h.x.toLong.toLong
  def u(c: S): 42 = c.h.x.toInt.unary_+
  def e(c: S): 42 = c.x.toInt
object Final:
  final val fv = new S().h.x.toInt
  val w: 42 = fv
@main def main(): Unit =
  val c = new S()
  println(Expect.d(c) + Expect.d2(c) + Expect.r(c) + Expect.a1(c) + Expect.a2(c) + Expect.a4(c) + Expect.a9(c))
  println(Expect.i(c) + Expect.t(c) + Expect.f(c) + Expect.p(c) + Expect.u(c) + Expect.e(c))
  println(Expect.d4(c) + Expect.w(c))
  println(Final.w)
  val k = new Holder()
  println(k.plus)
  println(k.identity)
