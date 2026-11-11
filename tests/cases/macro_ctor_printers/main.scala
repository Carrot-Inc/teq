class Box[T](val t: T)
class D[T](val a: T)(val b: Int)
class G6[T](val t: T):
  def this(t: T, s: String) = this(t)
class BN(a: => Int)(val b: Int)
class P(val a: Int, val b: Int)
type BI = Box[Int]
type BA[X] = Box[X]
type G6I = G6[Int]

object In:
  inline def mkE[T](x: T): Box[T] = new Box[T](x)
  inline def mkI[T](x: T): Box[T] = new Box(x)

object Main:
  def main(args: Array[String]): Unit =
    println("BoxE" + Re.pr(new Box[Int](1)))
    println("BoxI" + Re.pr(new Box(1)))
    println("DE" + Re.pr(new D[Int](1)(2)))
    println("DI" + Re.pr(new D(1)(2)))
    println("G6s" + Re.pr(new G6[Int](1, "x")))
    println("BN" + Re.pr(new BN(1)(2)))
    println("Pn" + Re.pr(new P(b = 1, a = 2)))
    println("BI" + Re.pr(new BI(1)))
    println("BA" + Re.pr(new BA(1)))
    println("G6I" + Re.pr(new G6I(1, "x")))
    println(Re.tpt(new Box(1)))
    println(Re.tpt(new Box[Int](1)))
    println(Re.tpt(new BI(1)))
    println(Re.tpt(new BA(1)))
    println(Re.tpt(In.mkE("s")))
    println(Re.tpt(In.mkI("s")))
    println(Re.tpt(new java.lang.StringBuilder("x")))
