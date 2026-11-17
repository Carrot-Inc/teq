// An assignment operator on a `var` of a class that derives from `scala.Dynamic`: `d += 1` is
// `d.applyDynamic("+=")(1)`, since scalac types the application as written before the assignment form, also where
// the class has a `+` of its own.
import scala.language.dynamics

class Counter(val n: Int) extends Dynamic:
  def +(k: Int): Counter = { println(s"plus($k)"); new Counter(n + k) }
  def applyDynamic(name: String)(args: Any*): Unit = println(s"apply($name)(${args.mkString(",")})")

@main def run(): Unit =
  var d = new Counter(1)
  d += 1
  println(d.n)
