// An inline member's defaults through its export forwarder on a class's val: the forwarder's getters are
// the member's on that path read on the call's receiver (dotty's `Namer.addForwarder` forwards each getter,
// an inline parameter's inline with its body registered, `registerInlineInfo`), an inline parameter's
// expanded at each use, a strict one's evaluated once on the receiver (before, the default read the path on
// the caller's `this`).
class Strict(k: Int):
  inline def h(inline x: Int = { println("default"); 3 }): Int = x + x
  inline def g(x: Int = k): Int = x * 2
class Holder(val s: Strict):
  export s.{h, g}
@main def run(): Unit =
  val o = Holder(Strict(5))
  println(o.h())
  println(o.g())
  println(o.h(1))
