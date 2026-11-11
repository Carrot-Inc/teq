// scalac 3.8.4: all five rejected calls report E134 (no matching overload), without a type mismatch.
// expect: overload_common_formal_function.scala:13:16: error: None of the overloaded alternatives
// expect: overload_common_formal_function.scala:14:17: error: None of the overloaded alternatives
// expect: overload_common_formal_function.scala:15:17: error: None of the overloaded alternatives
// expect: overload_common_formal_function.scala:16:18: error: None of the overloaded alternatives
// expect: overload_common_formal_function.scala:22:18: error: None of the overloaded alternatives
// absent: type mismatch
// absent: <error>
object OverloadCommonFormalFunction:
  def g(a: Int, b: Int): Int = a
  def g(a: Boolean, b: Int): Int = b
  def h(x: Int): Int = x
  val method = g("m", h)
  val matched = g(true, h)
  val literal = g("m", (x: Int) => x)
  val selected = g(true, (x: Int) => x)
  def f(a: Int, b: Int => Int): Int = 0
  def f(a: Boolean, b: Int => Int): Int = 1
  def id[A](a: A): A = a
  def bool(a: Int): Boolean = true
  val accepted = f(true, id)
  val rejected = f(true, bool)
// expect: 5 errors found
