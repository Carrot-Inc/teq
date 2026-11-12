// A static enum's value read through a null receiver of its companion's
// type, a val and a def: scalac 3.8.4 gives the value without dereferencing
// the receiver (no NullPointerException), the def's effect kept.
enum Color { case Red, Green }
@main def main(): Unit =
  val c: Color.type = null.asInstanceOf[Color.type]
  try println(c.Red)
  catch case _: NullPointerException => println("NPE")
  def d: Color.type = { println("d"); null.asInstanceOf[Color.type] }
  try println(d.Green)
  catch case _: NullPointerException => println("NPE")
