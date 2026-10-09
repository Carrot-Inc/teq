// Duplicate imports and a class body's import used by a subclass's body.
object Lib { val a = 1; class B }
object Dup {
  import Lib.a
  import Lib.a
  def f = a
}
class Base {
  import Lib.B
  def b: B = new B
}
class Sub extends Base {
  def c = b
}
