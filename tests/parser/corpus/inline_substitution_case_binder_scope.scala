// teq: --inline-substitution
// A case of an `inline match` binds its binder for its guard and body, a call the walk hands to
// the retype path included: `held`'s local class captures `y`. scalac prints 6.
inline def held(x: Int): Int =
  class C:
    def go(): Int = x + x
  new C().go()
inline def outer(x: Any): Int =
  inline x match
    case y: Int => held(y)
    case _ => 0
@main def run(): Unit =
  val n = 3
  println(outer(n))
