// jars: fixtures
// std: lean scala-library
// A jar's anonymous class holding a case class with a type parameter, its companion and calls
// of both (cats' `catsSddDeferForFunction0`, which http4s' `uri".."` reaches at compile time):
// the classes of a local class are converted with it, and `$anon.this.Deferred` is the
// companion the typer derives.
import fix.shapes.LocalDefer

object Main:
  def main(args: Array[String]): Unit =
    var n = 0
    val f = LocalDefer.deferFunction0.defer { n += 1; () => n * 10 }
    println(n)
    println(f())
    println(f())
    println(n)
