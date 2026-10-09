// jars: scala-library
// std: scala-library
// A by-name parameter whose value is a function (`y: => (() => Long)`): its default's getter returns the function,
// which erases to `Function0` as the parameter's thunk does, and the caller passes a thunk calling the getter all the
// same, as scalac's does, on a value class, an object and a constructor; an unused default runs nothing, and a strict
// parameter of the function type takes the getter's value.
object Count:
  var n = 0
class V(val x: Long) extends AnyVal:
  def f(y: => (() => Long) = { Count.n += 1; () => x + Count.n }): Long = y() + y()
  def unused(y: => (() => Long) = { Count.n += 100; () => 0L }): Long = x
  def strict(y: () => Long = { Count.n += 1; () => x + Count.n }): Long = y() + y()
object Lib:
  def f(y: => (() => Long) = { Count.n += 1; () => 10L + Count.n }): Long = y() + y()
class C(y: => (() => Long) = { Count.n += 1; () => 20L + Count.n }):
  def twice: Long = y() + y()
object Main:
  def main(args: Array[String]): Unit =
    println(new V(10L).f())
    println(Count.n)
    println(new V(10L).unused())
    println(Count.n)
    println(new V(10L).strict())
    println(Count.n)
    println(Lib.f())
    println(Count.n)
    val c = new C()
    println(Count.n)
    println(c.twice)
    println(Count.n)
