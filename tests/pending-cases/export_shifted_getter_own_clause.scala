// Through a companion extension method, an export forwarder's parameter takes the flags of
// the member's parameter at its place in the zip (Namer.scala:1392-1398), so `W.f`'s `a`
// has `b`'s default, whose getter `f$default$2(s)(a)` takes `a`'s own clause: called for `a`
// it is applied to the receiver alone, a function `Int => Int` where an `Int` is required.
// scalac 3.8.4 rejects the program (E007). Reviewer #8's N12 of the workers exports helper.
class Ops(val s: String):
  def f(a: Int)(b: Int = 2, c: Int = 3): Int = a * 100 + b * 10 + c + s.length * 1000
object W:
  extension (s: String)
    def ops: Ops = Ops(s)
    export ops.f
@main def run(): Unit =
  println(W.f("xy")()(7, 8))
