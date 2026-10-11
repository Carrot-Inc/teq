// The names of the constructs that keep one apart from an expression's, read where the parser's
// threads wrote them (tests/split.sh builds this beside a generated file large enough for them):
// a class's and a given's self alias, `C.this`, a pattern's named fields, an interpolator of a
// pattern, a language import and inline annotations; and the std's own self alias, which a
// sorted view's reverse reads, its file parsed with the program's for the `SeqView` it names.
import scala.language.implicitConversions
import scala.annotation.{nowarn, switch}

trait Show:
  def show: String

class Outer(val n: Int):
  outer =>
  class Inner:
    def back: Outer = outer
    def same: Boolean = Outer.this eq outer

given shown: Show with
  me =>
  def show: String = "shown " + (me eq summon[Show])

final class VerPat(sc: StringContext):
  def unapplySeq(s: String): Option[Seq[String]] =
    val prefix = sc.parts.head
    if s.startsWith(prefix) then Some(Seq(s.drop(prefix.length))) else None

extension (sc: StringContext) def ver: VerPat = VerPat(sc)

case class Size(width: Int, height: Int)

case class Meters(m: Int)
implicit def metersOf(n: Int): Meters = Meters(n)

object Main:
  def describe(x: Any): String =
    x match
      case Size(width = w) => "width " + w
      case ver"v$rest" => "version " + rest
      case n: Int =>
        (n: @switch) match
          case 1 => "one"
          case _ => "other"
      case _ => (x.toString: @nowarn)

  def main(args: Array[String]): Unit =
    val o = new Outer(4)
    val i = new o.Inner
    println(i.back.n)
    println(i.same)
    println(summon[Show].show)
    println(describe(Size(2, 3)))
    println(describe("v1.2"))
    println(describe(1))
    println(describe(7))
    println(describe('c'))
    val m: Meters = 5
    println(m)
    val sorted: scala.collection.SeqView[Int] = Seq(3, 1, 2).view.sorted
    println(sorted.reverse.toList)
    println(sorted.reverse.reverse.toList)
    println(sorted.reverse.sorted.toList)
    println(sorted.reverse.sorted(using Ordering.Int.reverse).toList)
