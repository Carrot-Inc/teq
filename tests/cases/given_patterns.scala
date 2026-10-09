// `given T` as a pattern binds a given of the case's type, in a `match` case and as a `for`
// value definition (redis4cats' `given Log[Task] = mkLog(..)` before the calls that need it),
// also where a guard follows it; `summonFrom`'s `case given T` makes the summoned value a given
// in its body. The binder is named as scalac names an anonymous given.
import scala.compiletime.summonFrom

trait Log:
  def name: String

class Box[A](val a: A):
  def map[B](f: A => B): Box[B] = Box(f(a))
  def flatMap[B](f: A => Box[B]): Box[B] = f(a)
  def withFilter(p: A => Boolean): Box[A] = this

def need(using l: Log): Box[String] = Box(l.name)
def logName(using l: Log): String = l.name

given Log with
  def name = "outer"

inline def viaSummonFrom: String = summonFrom {
  case given Log => logName + "!"
  case _ => "none"
}

@main def main(): Unit =
  val r =
    for
      x <- Box(1)
      given Log = new Log { def name = "log" + x }
      s <- need
    yield s
  println(r.a)
  val guarded =
    for
      x <- Box(2)
      given Log = new Log { def name = "guarded" + x }
      if x > 0
      s <- need
    yield s
  println(guarded.a)
  val logs = List(new Log { def name = "a" }, new Log { def name = "b" })
  println(for given Log <- logs yield logName)
  val any: Any = new Log { def name = "matched" }
  println(any match
    case given Log => logName
    case _ => "none")
  println(viaSummonFrom)
