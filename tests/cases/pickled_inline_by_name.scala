// An inline method's by-name and inline parameters, pickled as the parameters' references: scalac
// expands teq's body at each call, a by-name argument evaluated where the body reads it.
object Lib:
  var log = ""
  def tick(s: String): Int = { log += s; s.length }
  inline def twice(x: => Int): Int = x + x
  inline def once(x: Int): Int = x + x
  inline def unrolled(inline n: Int, x: => Int): Int = inline if n > 1 then x + unrolled(n - 1, x) else x

object InlineByName:
  def main(args: Array[String]): Unit =
    println(Lib.twice(Lib.tick("ab")))
    println(Lib.once(Lib.tick("c")))
    println(Lib.unrolled(3, Lib.tick("d")))
    println(Lib.log)
