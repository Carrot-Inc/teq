import scala.util.NotGiven

class R[A](val n: Int)
class Q[A](val n: Int)
object R {
  given base: R[List[List[Int]]] = new R(9)
  given fallback: R[List[Int]] = new R(0)
}
object Q {
  given q[A](using r: R[A]): Q[A] = new Q(r.n)
}
object Test {
  given absent(using NotGiven[Q[List[List[Int]]]]): R[List[Int]] = new R(1)

  val warm = summon[R[List[Int]]]
  val result = summon[Q[List[Int]]].n
}
@main def main(): Unit = println(Test.result)
