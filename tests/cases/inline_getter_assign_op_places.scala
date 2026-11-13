// Assignment operators through inline getters at the kinds of place a statement can stand: a top-level val
// and lazy val, a method, a default argument, a superclass argument, a class's statement and field, a local
// def, a lambda, an object's body, a by-name argument, a guard, `try` and `finally`, a `while`.
package meters

class Meter:
  private var held = 0
  inline def level: Int = held
  def level_=(v: Int): Unit = held = v

val shared = Meter()
val atStart: Int = { shared.level += 1; shared.level }
lazy val later: Int = { shared.level += 1; shared.level }

def inMethod(m: Meter): Unit = m.level += 1
def withDefault(m: Meter)(seen: Int = { m.level += 1; m.level }): Int = seen
class Base(val seen: Int)
class Sub(m: Meter) extends Base({ m.level += 1; m.level }):
  m.level += 1
  val field: Int = { m.level += 1; m.level }
object Body:
  shared.level += 1
  val seen: Int = shared.level
def byName(body: => Unit): Unit = body

@main def run(): Unit =
  val m = Meter()
  inMethod(m)
  val d = withDefault(m)()
  val s = Sub(m)
  def local(): Unit = m.level += 1
  local()
  val f = () => m.level += 1
  f()
  byName(m.level += 1)
  m.level match
    case n if { m.level += 1; n > 0 } => ()
    case _ => ()
  try m.level += 1 finally m.level += 1
  var i = 0
  while i < 2 do { m.level += 1; i += 1 }
  println(s"$atStart $later ${Body.seen} $d ${s.seen} ${s.field} ${m.level}")
