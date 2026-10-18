class Fn { def apply(x: Int): Int = x }
class IFn { infix def apply(x: Int): Int = x }
class C {
  def f: Int => Int = x => x
  def g: Fn = new Fn
  def h: IFn = new IFn
  def k(using s: String): Int => Int = x => x
  val v: Int => Int = x => x
  def m(x: Int): Int = x
}
extension (c: C) def e: Int => Int = x => x
extension (c: C) def e2(x: Int): Int = x
object P {
  given String = ""
  val c = new C
  val a = c f 1
  val b = c g 1
  val d = c h 1
  val i = c k 1
  val j = c v 1
  val l = c m 1
  val n = c e 1
  val o = c e2 1
}
