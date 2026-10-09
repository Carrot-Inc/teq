// A lexical extension beside an extension of the implicit scope is selected by dotty's prefix,
// `f(qual)` (`extMethodApply`): its receiver and the using clauses before the method's own
// parameters. Without an explicit list every using clause of the method is the prefix's, so a
// missing or ambiguous given there selects the companion's; a using clause after an explicit
// list is the application's, resolved after its arguments, whose type may settle it.
trait Need
trait Other
class R
object R:
  extension (r: R) def nullary: String = "companion nullary"
  extension (r: R) def ambiguous: String = "companion ambiguous"
  extension (r: R) def leading(x: Int): String = "companion leading"
  extension (r: R) def supplied(x: Int): String = "companion supplied"
  extension (r: R) def fed[A](a: A): String = "companion fed"
  extension (r: R) def wins(x: Int): String = "companion wins"

trait Evidence[A]:
  def value: String
object Evidence:
  given Evidence[String] with
    def value: String = "evidence String"

object Main:
  given o1: Other = new Other {}
  given o2: Other = new Other {}
  given Int = 3
  extension (r: R) def nullary(using Need): String = "lexical nullary"
  extension (r: R) def ambiguous(using Other): String = "lexical ambiguous"
  extension (r: R)(using Need) def leading(x: Int): String = "lexical leading"
  extension (r: R) def supplied(using n: Need)(x: Int): String = "lexical supplied " + x
  extension (r: R) def fed[A](a: A)(using e: Evidence[A]): String = e.value
  extension (r: R)(using n: Int) def wins(x: Int): String = "lexical wins " + (x + n)

  def main(args: Array[String]): Unit =
    val r = new R
    println(r.nullary)
    println(r.ambiguous)
    println(r.leading(1))
    println(r.supplied(using new Need {})(2))
    println(r.fed("hello"))
    println(r.wins(4))
