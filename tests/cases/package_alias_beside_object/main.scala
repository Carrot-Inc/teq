// A package's alias that shares its name with an object of the package is the type a wildcard
// import finds, the object the term.
import pz.*

object Main:
  def a(t: Trace): Int = t.length
  def d(x: Duration): Long = x + 1
  def u(x: UIO[Int]): Int = x.getOrElse(0)
  def main(args: Array[String]): Unit =
    println(a(Trace.empty))
    println(d(Duration.Zero))
    println(u(Some(3)))
