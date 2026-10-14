// A local trait's member that captures a local of its enclosing body is lifted (its proxies passed
// before its own arguments, dotty's LambdaLift), which the walker runs (`invoke_lifted`): the
// interpreter declines its caller to the walker, every call counted (tests/fold.sh asserts the
// statistics' "lifted member 1 (3 calls)" at the first call's promotion).
object Main:
  def run(n: Int): Int =
    val base = n * 10
    trait T:
      def value(i: Int): Int = i + base
    val t: T = new T {}
    t.value(n)
  def main(args: Array[String]): Unit =
    println(run(1))
    println(run(2))
    println(run(3))
