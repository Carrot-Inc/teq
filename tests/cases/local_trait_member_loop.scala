// A loop over a call of a local trait's member that captures nothing: the enclosing function is
// lowered with its loop, no unit declined (tests/fold.sh asserts it at the first call's promotion).
object Main:
  def run(n: Int): Int =
    trait T:
      def value(i: Int): Int = i + 1
    val t: T = new T {}
    var i = 0
    var total = 0
    while i < n do
      total += t.value(i)
      i += 1
    total
  def main(args: Array[String]): Unit =
    println(run(20)); println(run(20)); println(run(20))
