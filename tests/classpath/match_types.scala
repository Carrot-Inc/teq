// jars: scala-library
// The match types of scala-library's `Tuple` (`Zip`, `Head`, `Concat`, `Size`) load and reduce,
// so the tuple members typed by them check.
object Main:
  def main(args: Array[String]): Unit =
    val z: Tuple.Zip[(Int, String), (Int, String)] = (1, "a").zip((2, "b"))
    val h: Tuple.Head[(Int, String)] = (1, "a").head
    val t: Tuple.Tail[(Int, String)] = (1, "a").tail
    val c: Tuple.Concat[(Int, String), (Boolean, Double)] = (1, "a") ++ (true, 2.5)
    val n: Tuple.Size[(Int, String, Boolean)] = 3
    val e: Tuple.Elem[(Int, String, Boolean), 1] = "s"
    println(z)
    println(h)
    println(t)
    println(c)
    println(n)
    println(e)
