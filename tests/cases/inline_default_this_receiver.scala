// An inline method's inline parameter's default read as the argument of a later ordinary parameter's getter: the
// inline getter expands as its own call on the receiver, the `this` of its class that receiver (the inliner's
// `thisProxy`), at a direct call, on a receiver no path names, and through an export forwarder, whose getter's
// body is the member's getter on the forwarder's path (before, the default read the caller's `this`).
class Strict(k: Int):
  inline def f(inline x: Int = { println("sx"); k })(y: Int = x * 2): Int = x + y
class Holder(val s: Strict):
  export s.f
@main def run(): Unit =
  val s = Strict(5)
  println(s.f()())
  println(s.f()(1))
  println(s.f(3)())
  val o = Holder(Strict(7))
  println(o.f()())
  println(Strict(9).f()())
