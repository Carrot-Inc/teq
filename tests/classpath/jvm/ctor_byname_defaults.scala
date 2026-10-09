// jars: scala-library
// std: scala-library
// A by-name constructor parameter's default getter returns the value, the caller passing a thunk that calls it
// each time the parameter is read.
var count = 0
def tick(): Int = { count += 1; count }
class C(x: => Int = tick() * 7):
  def twice: Int = x + x
case class D(y: Int)(z: => Int = y + tick())
object Main:
  def main(args: Array[String]): Unit =
    val c = new C()
    println(count)
    println(c.twice)
    println(count)
    println(new C(1).twice)
