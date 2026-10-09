trait Box { type V }
class IB extends Box { type V = Int }
type With[A] = Box { type V = A }

class TC[A](val n: Int)
object TC { given TC[IB] = new TC(0) }

trait G[A] {
  given g[T <: With[A]]: TC[T] = new TC(1)
}

object S extends G[String] { val warm = summon[TC[IB]] }
object I extends G[Int] { val result = summon[TC[IB]].n }

@main def main(): Unit = println(I.result)
