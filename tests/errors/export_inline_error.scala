// expect: 9:33: error: bad
// An inline member's forwarder expands where it is called, with the call's arguments: the member's
// `error` for an argument its `inline if` rejects is reported at the call, as scalac reports it.
import scala.compiletime.error
class Lib:
  transparent inline def f(inline x: Int): Int = inline if x == 1 then 7 else error("bad")
val lib = new Lib
export lib.f
@main def run(): Unit = println(f(2))
