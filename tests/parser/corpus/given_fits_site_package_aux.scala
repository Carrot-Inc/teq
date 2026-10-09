class TC(val n: Int) { type T }
type Aux[A] = TC { type T = A }
object TC { given Aux[Int] = new TC(0) { type T = Int } }
trait G[A] { given g: Aux[A] = new TC(1) { type T = A } }

object S extends G[String] { val warm = summon[Aux[Int]] }
object I extends G[Int] { val result = summon[Aux[Int]].n }

@main def main(): Unit = println(I.result)
