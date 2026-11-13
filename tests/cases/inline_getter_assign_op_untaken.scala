// Assignment operators through inline getters at places no run takes (a false condition, a constant flag,
// a loop that never runs, after a `return` and a `throw`, an unused local def, lambda and lazy val, a case
// that cannot match, the other arm of a true condition) and in a branch this run does not take: each is
// compiled as where it is taken, and the program prints what scalac's does.
package meters

class Meter:
  private var held = 5
  inline def level: Int = held
  def level_=(v: Int): Unit = held = v
  inline def peak: Int = held
  def peak_=(v: Int): Unit = held = v

object Flags:
  final val Tracing = false
  inline val Checking = false

def early(m: Meter): Int =
  return 1
  m.level += 1
  2

def thrower(m: Meter): Int =
  if m.level < 0 then
    throw new IllegalStateException("below")
    m.level += 1
  3

@main def run(): Unit =
  val m = Meter()
  if false then m.level += 1
  if Flags.Tracing then m.level += 1
  if Flags.Checking then m.level += 1
  while false do m.peak += 1
  val calls = early(m) + thrower(m)
  def unusedLocal(): Unit = m.level += 1
  val unusedLambda = () => m.peak += 1
  (1: Any) match
    case _: Int => ()
    case _: String => m.level += 1
  lazy val unusedLazy: Unit = m.peak += 1
  if true then () else m.level += 1
  if m.level > 100 then m.peak += 1
  println(s"${m.level} ${m.peak} $calls")
