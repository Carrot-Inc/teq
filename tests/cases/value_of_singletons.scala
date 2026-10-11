// `ValueOf` of a singleton type as dotty's `Synthesizer.synthesizedValueOf` makes it: a
// constant's, `Unit`'s, a stable path's (a constant val's, a local val's) and a `this` type's.
object O:
  final val K: 7 = 7
  final val F = 1.0f
class Box(val n: Int):
  def me: Box = valueOf[this.type]

@main def run(): Unit =
  println(valueOf[7])
  println(valueOf[O.K.type])
  println(valueOf[O.F.type] == 1.0f)
  println(valueOf[Unit] == ())
  val x: String = "x"
  println(valueOf[x.type])
  val b = new Box(3)
  println(b.me.n)
  println(valueOf[O.type] eq O)
