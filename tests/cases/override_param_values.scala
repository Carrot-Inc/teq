// teq: --werror
// An `override val` parameter whose superclass (or enum) was passed another value: each class's own value, as scalac
// prints it. On JavaScript and in the interpreter the override shares the property or slot it overrides: the class's own
// parameters are written first, as scalac's constructors write them before the superclass's, and the superclass's
// constructor (an enum's `$init`) leaves the one a subclass's parameter overrides.
enum Changed(val s: Int):
  case Plus(override val s: Int) extends Changed(s + 1)
  case Other(override val s: Int, other: Int) extends Changed(other)

class A(val s: Int)
class B(override val s: Int) extends A(s + 1)
class C(s: Int) extends A(s)
class E(override val s: Int) extends C(99)

class Default(val s: Int = 1)
trait T extends Default
class TC(override val s: Int) extends T

@main def run(): Unit =
  println(Changed.Plus(4).s)
  println(Changed.Other(5, 6).s)
  println(B(1).s)
  println(E(3).s)
  println(TC(9).s)
