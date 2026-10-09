// A method converted to a single abstract method with a by-name parameter receives the argument
// unevaluated: the method evaluates it as often as its body reads it.
trait S:
  def f(x: => Int): Int

def id(x: => Int): Int = x
def twice(x: => Int): Int = x + x
def never(x: => Int): Int = 0
def strict(x: Int): Int = x * 10

@main def main(): Unit =
  var reads = 0
  def next(): Int = { reads += 1; reads }
  val a: S = id
  println(a.f(1))
  val b: S = twice
  println(s"${b.f(next())} $reads")
  val c: S = never
  println(s"${c.f(next())} $reads")
  val d: S = strict
  println(s"${d.f(next())} $reads")
