// A projection conforms to a projection of the same member through a prefix it conforms to,
// type lambdas compared up to the names of their parameters.
trait Box[F[_]]:
  type T
trait A:
  type T
trait B extends A

def same(x: Box[[X] =>> List[X]]#T): Box[[Y] =>> List[Y]]#T = x
def up(x: B#T): A#T = x

@main def main(): Unit =
  val xs: List[Box[[X] =>> List[X]]#T] = Nil
  println(xs.map(same))
  val ys: List[B#T] = Nil
  println(ys.map(up).size)
