// An assignment operator through an inline getter whose setter is inline too: both are expanded.
package meters

class Meter:
  private var held = 0
  inline def level: Int = held
  inline def level_=(v: Int): Unit = held = v

@main def run(): Unit =
  val m = Meter()
  m.level = 5
  m.level += 1
  println(m.level)
