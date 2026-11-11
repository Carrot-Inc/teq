// An enum's value read through a receiver with effects: dotty reads it through its companion's getter, whose
// receiver is always loaded, so a lazy val is forced and an object initialised before the value is read.
enum Color { case Red, Green, Blue }
object Holder { println("init Holder"); val c: Color.type = Color }
@main def m(): Unit =
  lazy val lz = { println("lz"); Color }
  println(lz.Red)
  println(Holder.c.Green)
  def d: Color.type = { println("d"); Color }
  println(d.Blue)
