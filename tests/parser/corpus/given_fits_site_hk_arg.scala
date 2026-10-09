class TC[A](val n: Int)
object TC { given TC[List[Int]] = new TC(0) }
trait G[A] { given g[F[_]]: TC[F[A]] = new TC(1) }

object S extends G[String] { val warm = summon[TC[List[Int]]] }
object I extends G[Int] { val result = summon[TC[List[Int]]].n }

@main def main(): Unit = println(I.result)
