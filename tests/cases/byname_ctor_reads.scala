// A by-name constructor parameter read in the constructor's code reads the thunk parameter (`Constructors.intoConstr`
// after `ElimByName`), a closure's and a lambda's of a SAM type capturing it: each read calls the thunk once, as
// scalac's do, and a method's read calls the one its field holds.
abstract class Fn:
  def apply(): Int
var count = 0
class A(s: => Int):
  val a = s
  val b = s
  val f = () => s
  val g: Fn = () => s
  def later = s
class B(s0: Int) extends A({ count += 1; count * 10 + s0 })

@main def run(): Unit =
  val x = B(1)
  println(s"${x.a} ${x.b} ${count}")
  println(s"${x.f()} ${x.g()} ${x.later} ${count}")
