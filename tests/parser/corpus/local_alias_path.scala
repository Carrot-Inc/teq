// A type alias local to a block over a member of a value the block defined before it.
trait Box:
  type T
  val v: T
def f(xs: List[Box]): List[String] = xs.map { x =>
  val et: Box = x
  type Elem = et.T
  def get: Elem = et.v
  get.toString
}
@main def main(): Unit = println(f(List(new Box { type T = Int; val v = 3 })))
