// A pattern definition whose right side's results are each a tuple literal of the pattern's arity,
// through a match, an `if` and a block (`Desugar.makePatDef`'s `SimpleTuple`, decided on the untyped
// right side by `forallResults(rhs, isMatchingTuple)`, TreeInfo.scala 359): a holder of the right
// side and a val per variable reading its element, a member's and a local's alike, whatever the
// workers' order; the build is the same at every thread count.
package tuplematchpatdef

object Amounts:
  def split(body: String): (String, String) =
    val (whole, fraction) = body.indexOf(".") match
      case -1 => (body, "")
      case i => (body.substring(0, i), body.substring(i + 1))
    (whole, fraction)

  def sign(n: Int): (String, Int) =
    val (mark, magnitude) = if n < 0 then ("-", -n) else { val m = n; ("+", m) }
    (mark, magnitude)

  inline def place(inline negative: Boolean): (String, Int) =
    val (mark, rank) = inline if negative then (("-", 1)) else ("+", 0)
    (mark, rank)

  val (unit, scale) = "EUR".length match
    case 3 => (("EUR", 100))
    case _ => ("?", 1)
