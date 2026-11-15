// A top-level inline method making an anonymous class over its argument,
// called in a class's constructor code with an argument reading the class's
// member (`wrapN(s)` by name, `wrapI(s + 1)` inline): the class reads the
// accessor through the instance under construction (scalac `2 3 5 6`).
abstract class Fn:
  def apply(): Int
inline def wrapN(x: => Int): Fn = new Fn { def apply() = x }
inline def wrapI(inline x: Int): Fn = new Fn { def apply() = x }
class A(val s: Int):
  val n = wrapN(s)
  val i = wrapI(s + 1)
class B(override val s: Int) extends A(s + 10)
@main def run(): Unit = println(s"${B(2).n()} ${B(2).i()} ${A(5).n()} ${A(5).i()}")
