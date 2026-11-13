// The receiver of an assignment operator through an inline getter is evaluated once, whether it is a call or
// a block (dotty's `LiftComplex.liftAssigned`).
package meters

class Meter:
  private var held = 5
  inline def level: Int = held
  def level_=(v: Int): Unit = held = v

var made = 0
def fresh(m: Meter): Meter = { made += 1; m }

@main def run(): Unit =
  val m = Meter()
  fresh(m).level += 1
  val first = s"${m.level} $made"
  made = 0
  { made += 1; m }.level += 2
  println(s"$first ${m.level} $made")
