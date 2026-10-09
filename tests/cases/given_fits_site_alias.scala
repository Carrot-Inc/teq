class TC[A](val n: Int)
object TC { given TC[List[Int]] = new TC(0) }
type W[A] = TC[List[A]]
trait G[A] { given g: W[A] = new TC(1) }

object S extends G[String] { val warm = summon[W[Int]] }
object I extends G[Int] { val result = summon[W[Int]].n }

@main def main(): Unit = println(I.result)
