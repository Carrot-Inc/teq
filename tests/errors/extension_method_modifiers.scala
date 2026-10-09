// expect: 11:35: error: abstract method h may not have `final` modifier
// expect: 14:11: error: value f is not a member of String
// expect: 15:11: error: value g is not a member of String
// An extension method keeps the modifiers written before it, in the one-method form too: a
// protected one is no candidate outside its object, and an abstract one may not be final.
object E:
  extension (s: String) protected def f(i: Int): Int = 1
  extension (s: String)
    protected def g(i: Int): Int = 2
trait T:
  extension (s: String) final def h: Int
object Main:
  import E.*
  val a = "".f(1)
  val b = "".g(1)
