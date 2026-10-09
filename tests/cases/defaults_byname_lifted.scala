// A by-name parameter's default in a call that names a later argument: the default's getter is
// the argument, evaluated at each use, never a value bound before the call.
object DefaultsByname {
  def f(x: => Int = { print("D"); 1 }, y: Int): Int = x + x
  def main(args: Array[String]): Unit = println(f(y = 0))
}
