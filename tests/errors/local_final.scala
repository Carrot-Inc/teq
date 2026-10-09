// expect: 7:15: error: The final modifier is not allowed on local definitions
// expect: 8:15: error: The final modifier is not allowed on local definitions
// expect: 9:15: error: The final modifier is not allowed on local definitions
// scalac's E200: `final` is not allowed on a local val, var or def; a local class or object may be final.
object Main:
  def main(args: Array[String]): Unit =
    final val x = 1
    final var v = 2
    final def f = x + v
    final class L
    final object O
    println(f)
