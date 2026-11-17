import lib.*
object Main:
  def main(args: Array[String]): Unit =
    val ones: Any = Array(Tuple1(1)); val pairs: Any = Array((1, "a"))
    println("pair " + List(ones, pairs).map(x => pairLength(x)).mkString(" "))
    println("one " + List(ones, pairs).map(x => oneLength(x)).mkString(" "))
