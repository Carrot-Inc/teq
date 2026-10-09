// expect: 9:9: error: class B needs to be abstract, since def f(c: Ctx)(x: c.T): String in trait A is not defined
// Parameters are matched by position, and a later one's type still has to be the same: `x: Int`
// does not implement `x: c.T`.
trait Ctx:
  type T
trait A:
  def f(c: Ctx)(x: c.T): String
object Main:
  class B extends A:
    def f(d: Ctx)(x: Int): String = "b"
