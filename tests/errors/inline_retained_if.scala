// expect: 10:45: error: Cannot reduce `inline if` because its condition is not a constant value: b
// expect: inlined from tests/errors/inline_retained_if.scala:10
// expect: 1 error found
// An inline method that implements a member that is not inline keeps a method for dynamic
// dispatch, which scalac 3.8.4 makes by inlining the body into itself with its own parameters:
// there an `inline if` on a parameter is no constant, reported at the body, called or not, with
// the inline stack trace of that inlining. A call on the class itself still reduces.
trait Choice { def pick(b: Boolean): Int }
class Fixed extends Choice:
  inline def pick(b: Boolean): Int = inline if b then 1 else 2

@main def run(): Unit = println(new Fixed().pick(true))
