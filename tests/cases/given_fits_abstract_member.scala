class TC[A](val n: Int)
object TC { given TC[Int] = new TC(0) }
trait G { type A; given g: TC[A] = new TC(1) }
object S extends G { type A = String }
object I extends G { type A = Int }

object Warm { import S.g; val warm = summon[TC[Int]] }
@main def main(): Unit = {
  import I.g
  println(summon[TC[Int]].n)
}
