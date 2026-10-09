// tupled and curried of function values of any arity, andThen/compose and Function.
package funconv

case class Big(a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int, h: Int)
case class Pt(x: Int, y: Int)
var calls = 0
def add(a: Int, b: Int): Int = a + b
def mk3(a: Int, b: String, c: Boolean): String = s"$a$b$c"
def f2(): (Int, Int) => Int =
  calls += 1
  (a, b) => a * b

@main def main(): Unit =
  val t = add.tupled
  println(t((1, 2)))
  println(add.curried(1)(2))
  val c = Big.apply.curried
  println(c(1)(2)(3)(4)(5)(6)(7)(8))
  println(Big.apply.tupled((1, 2, 3, 4, 5, 6, 7, 8)))
  println(List((1, 2), (3, 4)).map(Pt.apply.tupled))
  println(List((1, "b", true)).map(mk3.tupled))
  val g = f2().tupled
  println(g((3, 4)) + g((1, 1)))
  println(calls)
  val h: ((Int, Int)) => Int = add.tupled
  println(h(5 -> 6))
  val k = ((a: Int, b: Int, c: Int) => a - b - c).curried
  println(k(10)(1)(2))
  println(Some(Pt.apply.curried).map(_(1)).map(_(2)))
  val s = Pt.apply.tupled andThen (_.x)
  println(s((7, 8)))
  println((Pt.apply.curried)(1)(2))
  println(Function.tupled(add)((2, 3)))
  println(Function.untupled(add.tupled)(2, 3))
  val inc = (x: Int) => x + 1
  val twice = (x: Int) => x * 2
  println((inc andThen twice)(3))
  println((inc compose twice)(3))
  println(Function.chain(List(inc, twice, inc))(1))
  println(Function.const(7)("ignored"))
  println(Function.uncurried(add.curried)(4, 5))
  val seven = ((a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int) => a + b + c + d + e + f + g)
  println(seven.curried(1)(2)(3)(4)(5)(6)(7))
  println(seven.tupled((1, 2, 3, 4, 5, 6, 7)))
  println(List((1, 2, 3, 4, 5, 6, 7)).map(seven.tupled))
