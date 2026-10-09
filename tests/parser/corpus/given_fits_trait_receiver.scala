class TC[A](val n: Int)
object TC { given TC[Int] = new TC(0) }
trait G[A] { given g: TC[A] = new TC(1) }

object S extends G[String] { val warm = summon[TC[Int]] }
object I extends G[Int] { val result = summon[TC[Int]].n }

@main def main(): Unit = println(I.result)
