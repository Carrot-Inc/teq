// `apply(..)` written bare inside a case class's companion that declares `apply`s of the same arity (indigo's
// `Animation.create`): the class's synthetic `apply` is among the alternatives, as `Desugar.classDef` adds it to the
// companion beside the written ones.
final case class Anim(key: String, label: Int, cycles: List[Int]):
  def size: Int = cycles.length

object Anim {
  def apply(key: String, frameOne: Double, frames: Double*): Anim =
    Anim(key, frameOne, frames.toList)

  def apply(key: String, frameOne: Double, frames: List[Double]): Anim =
    Anim(key, frameOne.toInt, frames.map(_.toInt))

  def create(key: String, n: Int): Anim =
    apply(key, n, List(n, n))
}

@main def run(): Unit =
  println(Anim.create("a", 3))
  println(Anim("b", 1.5, 2.5, 3.5).size)
