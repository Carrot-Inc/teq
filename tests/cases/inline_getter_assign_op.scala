// An assignment operator through a getter that is an inline method, with a setter beside it: `m.level += 1`
// is `m.level = m.level + 1`, the read expanded as a read written by hand is (dotty's `typedOpAssign` types
// the assignment over the one typed left operand).
package meters

class Meter:
  private var held = 0
  inline def level: Int = held
  def level_=(v: Int): Unit = held = v

@main def run(): Unit =
  val m = Meter()
  m.level = 5
  m.level += 1
  println(m.level)
