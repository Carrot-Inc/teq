// A `return` out of a closure in a def whose result is a value class: the closure throws the def's erased result (the
// underlying value, boxed where it is a primitive), which the def's handler takes back, as scalac's NonLocalReturns
// does after erasure. The closure is a lambda, a lambda in a lambda or a by-name argument; the def is an object's, a
// class's or a local one; and a result of Any or Option keeps the box. A local lazy val's initialiser:
// return_non_local_lazy_value_class.
import scala.annotation.nowarn

class Meters(val u: Int) extends AnyVal
class Big(val u: Long) extends AnyVal
class Ratio(val u: Double) extends AnyVal
class Name(val u: String) extends AnyVal
class Wrap[A](val u: A) extends AnyVal

def byName(f: => Unit): Unit = f

@nowarn object Defs {
  def lambda(xs: List[Int]): Meters = { xs.foreach(x => if x > 1 then return Meters(x)); Meters(0) }
  def nested(xs: List[Int]): Meters = { xs.foreach(y => List(y).foreach(x => if x > 1 then return Meters(x * 10))); Meters(0) }
  def byNameArg(n: Int): Meters = { byName { if n > 1 then return Meters(n * 100) }; Meters(0) }
  def long(xs: List[Int]): Big = { xs.foreach(x => if x > 1 then return Big(x * 10000000000L)); Big(0L) }
  def double(xs: List[Int]): Ratio = { xs.foreach(x => if x > 1 then return Ratio(x + 0.5)); Ratio(0.25) }
  def string(xs: List[Int]): Name = { xs.foreach(x => if x > 1 then return Name("n" + x)); Name("none") }
  def generic(xs: List[Int]): Wrap[Int] = { xs.foreach(x => if x > 1 then return Wrap(x)); Wrap(0) }
  def any(xs: List[Int]): Any = { xs.foreach(x => if x > 1 then return Meters(x)); 0 }
  def option(xs: List[Int]): Option[Meters] = { xs.foreach(x => if x > 1 then return Some(Meters(x))); None }

}

@nowarn class Holder(k: Int) {
  def lambda(xs: List[Int]): Meters = { xs.foreach(x => if x > 1 then return Meters(x + k)); Meters(k) }
}

@nowarn object Locals {
  def lambda(xs: List[Int]): Meters = {
    def inner(): Meters = { xs.foreach(x => if x > 1 then return Meters(x)); Meters(0) }
    Meters(inner().u + 1000)
  }
}

object Main {
  def main(args: Array[String]): Unit = {
    val hit = List(1, 2, 3)
    val miss = List(0, 1)
    println("lambda " + Defs.lambda(hit).u + " " + Defs.lambda(miss).u)
    println("nested " + Defs.nested(hit).u + " " + Defs.nested(miss).u)
    println("by-name " + Defs.byNameArg(2).u + " " + Defs.byNameArg(0).u)
    println("long " + Defs.long(hit).u + " " + Defs.long(miss).u)
    println("double " + Defs.double(hit).u + " " + Defs.double(miss).u)
    println("string " + Defs.string(hit).u + " " + Defs.string(miss).u)
    println("generic " + Defs.generic(hit).u + " " + Defs.generic(miss).u)
    println("any " + (Defs.any(hit) match { case m: Meters => "Meters " + m.u; case o => "other " + o }) + " " + Defs.any(miss))
    println("option " + Defs.option(hit).map(_.u) + " " + Defs.option(miss).map(_.u))
    val h = Holder(5)
    println("class " + h.lambda(hit).u + " " + h.lambda(miss).u)
    println("local " + Locals.lambda(hit).u + " " + Locals.lambda(miss).u)
  }
}
