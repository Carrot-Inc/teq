opaque type O = Int

class TC[A](val n: Int)
object TC { given TC[Int] = new TC(0) }

class R
object R { given R = new R }

given bad[T](using TC[T], T =:= String, TC[Int], Nothing): R = new R

def run[A](f: TC[A] ?=> Int)(using tc: TC[A]): Int = f(using tc)

@main def main(): Unit = println(run {
  val warm = summon[R]
  summon[TC[Int]].n
}(using new TC[Int](2)))
