// A call of a local trait's member that captures nothing runs as any method call does, so the
// interpreter lowers its caller (tests/fold.sh asserts that no unit is declined at the first call's
// promotion); a member that captures a local is lifted and its caller declined, as in
// tests/cases/local_trait_member_lifted.scala.
object Main:
  def run(n: Int): Int =
    trait T:
      def value(i: Int): Int = i + 1
    val t: T = new T {}
    t.value(n)
  def main(args: Array[String]): Unit =
    println(run(1))
    println(run(2))
    println(run(3))
