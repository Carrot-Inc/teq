// Pattern definitions in a class or object body (`lazy val (first, last) = ..`, `val (lo, hi): (Int,
// Int) = ..`, `var (n, m) = ..`), desugared as scalac's `makePatDef` does: a private holder of the
// match and a member per variable.
final case class Info(name: String, first: Option[String], last: Option[String]):
  lazy val (firstFor, lastFor) = first.zip(last) match
    case Some((f, l)) if f.nonEmpty => (f, Some(l))
    case _ =>
      val words = name.trim.split(" ", 2)
      (words.headOption.getOrElse(""), words.tail.headOption)

object Consts:
  val (lo, hi): (Int, Int) = (1, 10)
  private val Some(single) = Option(42): @unchecked
  val List(a, b, c) = List("x", "y", "z"): @unchecked
  def sum = lo + hi + single

class Counter:
  var (n, m) = (0, 1)
  def bump(): Unit = { n += 1; m *= 2 }

@main def main(): Unit =
  val i = Info("Ada Lovelace", None, None)
  println((i.firstFor, i.lastFor))
  val j = Info("x", Some("Grace"), Some("Hopper"))
  println((j.firstFor, j.lastFor))
  println((Consts.lo, Consts.hi, Consts.sum, Consts.a + Consts.b + Consts.c))
  val k = Counter()
  k.bump(); k.bump()
  println((k.n, k.m))
