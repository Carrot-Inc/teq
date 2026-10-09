object Models:
  final case class Meta(count: Int)

object Outer:
  object Inner:
    final class Leaf

@main def run(): Unit =
  println(Macros.prefixOf[Models.Meta])
  println(Macros.prefixOf[Outer.Inner.Leaf])
