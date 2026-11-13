// An assignment operator on an inline getter named without a qualifier, inside its class and inside a class
// nested in it: the receiver is `this`, or the enclosing instance.
package meters

class Meter:
  private var held = 5
  inline def level: Int = held
  def level_=(v: Int): Unit = held = v
  def bump(): Int = { level += 1; level }
  class Step:
    def again(): Int = { level += 1; level }

@main def run(): Unit =
  val m = Meter()
  val first = m.bump()
  println(s"$first ${m.Step().again()}")
