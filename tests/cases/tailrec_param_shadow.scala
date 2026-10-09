// A tail-recursive loop whose body binds a local named like a parameter (`case acc =>`): the
// JavaScript copy of the parameter and the local are named apart.
object Main:
  def loop(i: Int, acc: Int): Int =
    if i >= 5 then acc
    else (acc + i) match
      case n if n > 100 => n
      case acc => loop(i + 1, acc)
  def main(args: Array[String]): Unit =
    println(loop(0, 0))
