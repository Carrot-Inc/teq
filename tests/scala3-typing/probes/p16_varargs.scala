object T:
  def foo(x: String*, y: String): Int = 1
  def bar(x: Int*)(y: Int*): Int = 2
case class ByName(x: => Int)
class C(val x: => Int)
@main def run(): Unit = println(T.foo("a", y = "b"))
