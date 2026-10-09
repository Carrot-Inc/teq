class TC[A](val n: Int) { type V }
type Mark[A] = TC[A] { type V = Int }
object TC { given Mark[Int] = new TC[Int](0) { type V = Int } }
trait G[A] { given g: Mark[A] = new TC[A](1) { type V = Int } }

object S extends G[String] { val warm = summon[Mark[Int]] }
object I extends G[Int] { val result = summon[Mark[Int]].n }

@main def main(): Unit = println(I.result)
