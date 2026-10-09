class TC[A, B](val n: Int)
object TC { given TC[Int, Int] = new TC(0) }
type Wrap[A] = TC[A, A]
trait G[A] { given g: Wrap[A] = new TC(1) }

object S extends G[String] { val warm = summon[Wrap[Int]] }
object I extends G[Int] { val result = summon[Wrap[Int]].n }

@main def main(): Unit = println(I.result)
