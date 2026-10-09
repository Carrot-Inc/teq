// An inline expansion whose result is a node another worker made: `summonInline` gives the
// reference to the given the search found before the fork (`warm` primed it), which the walk
// types in its own instance's table and leaves as it is, the records of the expansion on it as
// one worker's are.
case class Box(i: Int)
object Main:
  transparent inline def show[A](i: Int): String =
    val m = scala.compiletime.summonInline[scala.deriving.Mirror.Of[A]]
    m.toString + ":" + i + ":" + (i + 1) + ":" + (i + 2)
  val warm = show[Box](0)
  def first(i: Int): String = show[Box](i)
  def second(i: Int): String = show[Box](i)
  def main(args: Array[String]): Unit =
    println(first(args.length))
    println(second(args.length))
