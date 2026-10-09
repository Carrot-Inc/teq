// Adapted from scala3 tests/run/t7584.scala (Apache-2.0, see tests/scala3/README.md).
// Test case added to show the behaviour of functions with
// by-name parameters.  The evaluation behaviour was already correct.
//
// We did flush out a spurious "pure expression does nothing in statement position"
// warning, hence -Xfatal-warnings in the flags file.
object Test {
  def main(args: Array[String]): Unit = ()
  def foo(f: (=> Int, => Int) => Unit) = f({println("a"); 0}, {println("b"); 1})
  println("no calls")
  foo((a, b) => ())
  println("call A")
  foo((a, b) => a)
  println("call B twice")
  foo((a, b) => {b; b})
}
