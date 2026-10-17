// teq: --werror
// Pattern definitions scalac 3.8.4 checks without a word (`Checking.checkIrrefutable`): a sequence's
// extractor, `List.unapplySeq`'s or `Seq`'s, is irrefutable whatever the elements asked
// (`SpaceEngine.isIrrefutable`, its result has an element type); a tuple, a product with `isEmpty:
// false`; and refutable ones marked `.runtimeChecked` (a block ending in it too, whose type is its
// result's) or `: @unchecked`, a match on `.runtimeChecked` too; members and locals alike.
package patternbindingirrefutable

object DisplayModels:
  final case class DisplayCard(name: String, price: Option[String] = None)
object Cards:
  import DisplayModels.*
  def cards(names: List[String]): List[DisplayCard] = names.map(n => DisplayCard(n))
final case class Pair(n: Int, s: String):
  def isEmpty: false = false
  def get: Pair = this
object Prod:
  def unapply(s: String): Pair = Pair(s.length, s)
object Ex:
  def unapply(s: String): Option[Int] = if s.nonEmpty then Some(s.length) else None
object Members:
  import Cards.*
  val List(first) = cards(List("m"))
  val Seq(p, q) = Seq(1, 2)
  val h :: t = List(3, 4).runtimeChecked

@main def main(): Unit =
  import Cards.*
  val List(card) = cards(List("a"))
  val List(x, y) = cards(List("b", "c"))
  val Seq(s1, s2) = Seq(1, 2)
  val List(head, rest*) = List(5, 6, 7).runtimeChecked
  val (a, b) = (3, "t")
  val Prod(n, s) = "four"
  val h2 :: t2 = List(9).runtimeChecked
  val Some(o2) = Option(10).runtimeChecked
  val Ex(e2) = "eleven".runtimeChecked
  val Some(o3) = (Option(12): @unchecked)
  val Some(o4) = { println("once"); Option(14).runtimeChecked }
  val matched = Option(13).runtimeChecked match
    case Some(v) => v
  println(s"${card.name} ${x.name} ${y.name} $s1 $s2 $head $rest $a $b $n $s")
  println(s"$h2 $t2 $o2 $e2 $o3 $o4 $matched")
  println(s"${Members.first.name} ${Members.p} ${Members.q} ${Members.h} ${Members.t}")
