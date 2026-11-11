trait Source:
  type T
  type U = T
trait Concrete extends Source:
  type T = String
object Stable extends Concrete

type Field[X] = Source { type U = X }
type Extract[Y] = Y match
  case Field[x] => x

@main def run(): Unit =
  summon[Extract[Concrete] =:= String]
  summon[Extract[Stable.type] =:= String]
  println("ok")
