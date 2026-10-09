// `h *: t` patterns with typed binders under either std: the lean std's `*:` object has scala-library's `unapply`,
// found before the trait it accompanies, and the object is callable as a term.
object Main:
  def main(args: Array[String]): Unit =
    val t: Int *: String *: EmptyTuple = (1, "a")
    t match
      case (a: Int) *: (b: String) *: EmptyTuple => println(a.toString + b)
    val u: Tuple = (2, "b", 3.5)
    u match
      case (x: Int) *: rest => println(x + rest.toList.size)
      case _ => println("other")
    (3, "c") match
      case a *: (b: String) *: EmptyTuple => println(a + b.length)
    val r = *:.unapply(t)
    println(r)
