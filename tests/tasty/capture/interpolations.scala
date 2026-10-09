// Interpolations of any number of parts: the capture keeps which parts the concatenation
// leaves out, however many there are (scalac's pickle holds `StringContext.apply` with them all).
object Interpolations:
  def thirtyTwo(x: Int): String = s"$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x$x"
  def seventy(x: Int): String = s"$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-$x-"
  def raw1(x: Int): String = raw"a$x$x\n"

  def main(args: Array[String]): Unit =
    println(thirtyTwo(1).length)
    println(seventy(2).length)
    println(raw1(3))
