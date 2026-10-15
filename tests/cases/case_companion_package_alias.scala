// A case class without a written companion has one (dotty's `Desugar.classDef`): its term is that object,
// a value and a singleton type of its own (`val Texture: shared.Texture.type = shared.Texture`, indigo's
// package aliases), with the class's `apply`.
object shared:
  final case class Texture(name: String, n: Int)
  final case class Mouse(x: Int)
val Texture: shared.Texture.type = shared.Texture
val Mouse = shared.Mouse
@main def run(): Unit =
  println(Texture("a", 1))
  println(Mouse(3))
  println(List(("c", 3)).map(Texture.apply.tupled))
