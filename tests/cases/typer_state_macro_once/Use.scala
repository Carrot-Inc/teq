trait Ops:
  extension [T](x: Int) def combine(y: String): String = "s" + y
  extension [T](x: Int) def combine(y: Int): String = "i" + y

object O extends Ops

@main def run(): Unit =
  println(O.combine[Unit](1)(Counter.next()))
  println(Counter.next())
