class TC(val n: Int) { type T }
type IntTC = TC { type T = Int }
object TC { given IntTC = new TC(0) { type T = Int } }

trait G[A] {
  type Out = TC { type T = A }
  given g: Out = new TC(1) { type T = A }
}

object S extends G[String] { val warm = summon[IntTC] }
object I extends G[Int] { val result = summon[IntTC].n }

@main def main(): Unit = println(I.result)
