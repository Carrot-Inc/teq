package ncla

// A library's inline method whose body makes a class with a method that expands another inline call on `this`,
// whose body makes a class (with a lambda in its method) reading `this`: expanded downstream on a receiver that is
// bound before the body, the inner class holds that receiver.
trait Shower:
  def show(): String
trait Deriv:
  def name: String
  inline def leaf(): Shower = new Shower { def show(): String = { val f = (i: Int) => "leaf " + name + i; f(1) } }
  inline def derived: Shower = new Shower { def show(): String = leaf().show() }
  inline def passed: Shower = new Shower { def show(): String = Deriv.leafOf(Deriv.this).show() }
object Deriv:
  inline def leafOf(d: Deriv): Shower = new Shower { def show(): String = { val f = (i: Int) => "leafOf " + d.name + i; f(1) } }
class Impl(val name: String) extends Deriv
