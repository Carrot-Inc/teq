// A value class's method is called as its companion's extension method (dotty's
// `VCInlineMethods.rewire`): for a class with no type parameters the companion's instance comes
// first and the receiver is evaluated in place, so the companion is initialised before the
// receiver's effects. A generic class's receiver is bound to a temporary first (`evalOnce`) unless
// it is idempotent (`isIdempotentExpr`): an object's val, a path through objects, a lazy val and an
// export's forwarder of a val are, and go in place after the companion; a call and a parameterless
// def are not. Each case has a class of its own, so that its first call shows where the companion
// is initialised.
final class V(val u: Int) extends AnyVal { def plus(x: Int): Int = u + x }
object V { println("V companion") }
final class W[T](val t: T) extends AnyVal { def pair(x: Int): String = s"$t/$x" }
object W { println("W companion") }
final class Wo[T](val t: T) extends AnyVal { def pair(x: Int): String = s"$t/$x" }
object Wo { println("Wo companion") }
final class Wp[T](val t: T) extends AnyVal { def pair(x: Int): String = s"$t/$x" }
object Wp { println("Wp companion") }
final class Wl[T](val t: T) extends AnyVal { def pair(x: Int): String = s"$t/$x" }
object Wl { println("Wl companion") }
final class Wd[T](val t: T) extends AnyVal { def pair(x: Int): String = s"$t/$x" }
object Wd { println("Wd companion") }
final class We[T](val t: T) extends AnyVal { def pair(x: Int): String = s"$t/$x" }
object We { println("We companion") }
object Holds { println("Holds body"); val w: Wo[String] = new Wo("h") }
object Outer { object Inner { println("Inner body"); val w: Wp[String] = new Wp("i") } }
class Holder { def wd: Wd[String] = { println("wd"); new Wd("d") } }
// The arguments' forms, each with a class of its own: a default filled, arguments named out of order, varargs, a
// by-name argument, an argument that is itself such a call; the receiver a call (bound first by the lift) or an
// object's val.
final class Da[A](val value: A) extends AnyVal { def read(n: Int = 1): String = s"$value/$n" }
object Da { println("Da companion") }
final class Di[A](val value: A) extends AnyVal { def read(n: Int = 1): String = s"$value/$n" }
object Di { println("Di companion") }
final class Na[A](val value: A) extends AnyVal { def two(a: Int, b: Int): String = s"$value/$a/$b" }
object Na { println("Na companion") }
final class Ni[A](val value: A) extends AnyVal { def two(a: Int, b: Int): String = s"$value/$a/$b" }
object Ni { println("Ni companion") }
final class Va[A](val value: A) extends AnyVal { def all(xs: Int*): String = s"$value/${xs.sum}" }
object Va { println("Va companion") }
final class Bn[A](val value: A) extends AnyVal { def lazily(x: => Int): String = s"$value/$x" }
object Bn { println("Bn companion") }
final class Ne[A](val value: A) extends AnyVal { def plus(s: String): String = s"$value+$s" }
object Ne { println("Ne companion") }
final class Ma[A](val value: A) extends AnyVal { def read(n: Int = 1): String = s"$value/$n" }
object Ma { println("Ma companion") }
object HoldsDi { println("HoldsDi body"); val d: Di[Int] = new Di(2) }
object HoldsNi { println("HoldsNi body"); val n: Ni[Int] = new Ni(4) }
object Src { println("Src body"); val w: We[String] = new We("e") }
object Exports { export Src.w }
// A generic value class whose companion the program does not write, which the JVM writes and calls through: each
// receiver form in a method of its own, in place where it is idempotent as for a companion the program writes.
final class Wc[T](val t: T) extends AnyVal { def pair(x: Int): String = s"$t/$x"; def again: String = pair(0) }
object HoldsC { println("HoldsC body"); val w: Wc[String] = new Wc("c") }
object OuterC { object InnerC { println("InnerC body"); val w: Wc[String] = new Wc("p") } }
class HolderC { val wf: Wc[String] = new Wc("f"); def wd: Wc[String] = { println("wcd"); new Wc("d") } }
object CompanionLess {
  lazy val lc: Wc[String] = { println("lazy c"); new Wc("l") }
  def mkC(): Wc[String] = { println("mkC"); new Wc("m") }
  def param(w: Wc[String]): String = w.pair(1)
  def variable(): String = { var vc = new Wc("v"); vc.pair(2) }
  def field(h: HolderC): String = h.wf.pair(3)
  def define(h: HolderC): String = h.wd.pair(4)
  def literal(): String = new Wc("x").pair(5)
  def objectVal(): String = HoldsC.w.pair(6)
  def lazyVal(): String = lc.pair(7)
  def call(): String = mkC().pair(8)
  def path(): String = OuterC.InnerC.w.pair(9)
  def itself(): String = new Wc("t").again
  def cast(a: Any): String = a.asInstanceOf[Wc[String]].pair(10)
  def run(): Unit = {
    val h = new HolderC
    println(param(new Wc("a"))); println(variable()); println(field(h)); println(define(h)); println(literal())
    println(objectVal()); println(lazyVal()); println(call()); println(path()); println(itself()); println(cast(h.wf))
  }
}
object Main {
  lazy val lw: Wl[String] = { println("lazy w"); new Wl("l") }
  def mkV(): V = { println("mkV"); new V(2) }
  def mkW(): W[String] = { println("mkW"); new W("m") }
  def mk[A](tag: String, a: A): A = { println(tag); a }
  def arg(tag: String, v: Int): Int = { println(tag); v }
  def main(args: Array[String]): Unit = {
    println(mkV().plus(1))
    println(mkW().pair(1))
    println(Holds.w.pair(2))
    println(Outer.Inner.w.pair(3))
    println(lw.pair(4))
    println(new Holder().wd.pair(5))
    println(Exports.w.pair(6))
    println(mk("da", new Da(1)).read())
    println(HoldsDi.d.read())
    println(mk("na", new Na(3)).two(b = arg("b", 2), a = arg("a", 1)))
    println(HoldsNi.n.two(b = arg("b2", 2), a = arg("a2", 1)))
    println(mk("va", new Va(5)).all(arg("x", 1), arg("y", 2)))
    println(mk("bn", new Bn(6)).lazily(arg("lazy", 3)))
    println(mk("ne", new Ne(7)).plus(mk("ma", new Ma(8)).read()))
    println(mk("da2", new Da(9)).read(arg("n", 2)))
    CompanionLess.run()
  }
}
