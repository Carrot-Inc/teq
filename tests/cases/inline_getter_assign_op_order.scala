// The order of an assignment operator through an inline getter is the assignment's: the receiver once, the
// getter's read, the right-hand side, then the setter.
package meters

class Meter:
  private var held = 5
  inline def level: Int = { print("read "); held }
  def level_=(v: Int): Unit = { print("write "); held = v }

def receiver(m: Meter): Meter = { print("receiver "); m }
def amount(): Int = { print("amount "); 2 }

@main def run(): Unit =
  val m = Meter()
  receiver(m).level += amount()
  println(m.level)
