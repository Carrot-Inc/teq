// jars: quotenest-lib
// A library macro's quote pattern whose declared binding is bounded by the type an enclosing pattern bound
// (`type u <: `t``): the bound is that type, `Int`, which `1` conforms to and `"s"` does not.
import qnest.*

@main def run(): Unit =
  println(Bounded.test(1))
  println(Bounded.test("s"))
