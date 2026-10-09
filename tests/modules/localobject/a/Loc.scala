package loa

// A local object, which scalac pickles as its module class `Pair$` and a val of it, both
// `OBJECT`: an extractor and a counter a downstream module's run reads from the products.
object Loc:
  def pairs(xs: List[Any]): String =
    object Pair:
      val tag = "p"
      def unapply(t: Any): Option[(Any, Any)] = t match
        case (a, b) => Some((a, b))
        case _ => None
    xs.map {
      case Pair(a, b) => s"${Pair.tag}($a,$b)"
      case x => x.toString
    }.mkString(", ")
  def counter(): Int =
    object Count:
      var n = 0
      def bump(): Int =
        n += 1
        n
    Count.bump()
    Count.bump()
  inline def pairsInline(xs: List[Any]): String = pairs(xs)
