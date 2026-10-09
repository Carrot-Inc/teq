// Transparent inline methods, whose expansions keep the precise type, pickled bare: scalac's
// expansion of teq's body gives the type the call site uses.
object Picks:
  transparent inline def pick(inline flag: Boolean): Any = inline if flag then 1 else "one"
  transparent inline def first(t: (Int, String)): Int = t._1
  transparent inline def pair[A](a: A): (A, A) = (a, a)

object InlineTransparent:
  def main(args: Array[String]): Unit =
    val i: Int = Picks.pick(true)
    val s: String = Picks.pick(false)
    println(i + 1)
    println(s.length)
    println(Picks.first((4, "x")) * 2)
    val p: (String, String) = Picks.pair("z")
    println(p._1 + p._2)
