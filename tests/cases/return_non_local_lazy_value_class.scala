// A `return` out of a local lazy val's initialiser in a def whose result is a value class: the initialiser throws the
// def's erased result (the underlying value), which the def's handler takes back, as scalac's NonLocalReturns does
// after erasure. The return stands in the initialiser, in a lambda in it, in a val read twice, in a val in a lambda, in
// a lazy val in another's initialiser; the def is an object's, a class's or a local one; a value class over String too.
import scala.annotation.nowarn

class Meters(val u: Int) extends AnyVal
class Name(val u: String) extends AnyVal

@nowarn object Defs {
  def lazyDirect(n: Int): Meters = { lazy val v: Int = { if n > 1 then return Meters(n); 0 }; Meters(v + 1) }
  def lazyLambda(xs: List[Int]): Meters = { lazy val v: Int = { xs.foreach(x => if x > 1 then return Meters(x)); 0 }; Meters(v + 1) }
  def lazyTwice(n: Int): Meters = { lazy val v: Int = { if n > 1 then return Meters(n); 3 }; Meters(v + v) }
  def lazyInLambda(xs: List[Int]): Meters = { xs.foreach { x => lazy val v: Int = { if x > 1 then return Meters(x * 100); 0 }; v }; Meters(0) }
  def lazyInLazy(n: Int): Meters = { lazy val v: Int = { lazy val w: Int = { if n > 1 then return Meters(n); 0 }; w }; Meters(v + 1) }
  def lazyString(n: Int): Name = { lazy val v: String = { if n > 1 then return Name("n" + n); "none" }; Name(v) }
}

@nowarn class Holder(k: Int) {
  def lazyDirect(n: Int): Meters = { lazy val v: Int = { if n > 1 then return Meters(n + k); 0 }; Meters(v + k) }
}

@nowarn object Locals {
  def lazyDirect(n: Int): Meters = {
    def inner(): Meters = { lazy val v: Int = { if n > 1 then return Meters(n); 0 }; Meters(v) }
    Meters(inner().u + 1000)
  }
}

object Main {
  def main(args: Array[String]): Unit = {
    val hit = List(1, 2, 3)
    val miss = List(0, 1)
    println("lazy direct " + Defs.lazyDirect(2).u + " " + Defs.lazyDirect(0).u)
    println("lazy lambda " + Defs.lazyLambda(hit).u + " " + Defs.lazyLambda(miss).u)
    println("lazy twice " + Defs.lazyTwice(2).u + " " + Defs.lazyTwice(0).u)
    println("lazy in lambda " + Defs.lazyInLambda(hit).u + " " + Defs.lazyInLambda(miss).u)
    println("lazy in lazy " + Defs.lazyInLazy(2).u + " " + Defs.lazyInLazy(0).u)
    println("lazy string " + Defs.lazyString(2).u + " " + Defs.lazyString(0).u)
    val h = Holder(5)
    println("class " + h.lazyDirect(2).u + " " + h.lazyDirect(0).u)
    println("local " + Locals.lazyDirect(2).u + " " + Locals.lazyDirect(0).u)
  }
}
