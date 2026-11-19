// Up to Scala 3.5 the more specific of two givens wins (dotty's `Applications.compare`, `CompareScheme.Intermediate`
// where `sourceVersion.isAtMost(3.5)`): of a given of `A` and one of `B <: A`, of a given of `A` and an implicit of
// `B`, and of an alias of `A & B` against a given object of `A`, the specific one; a given object of two parents
// against an alias of a parent of theirs (`P & Q`, `R <: Q`) likewise. scalac 3.8.4 prints `specific`, `specific`, `1`
// and `1`.
import scala.language.`3.5`
class A(val tag: String); class B(tag: String) extends A(tag)
object Givens:
  given general: A = A("general")
  given specific: B = B("specific")
object Mixed:
  given general: A = A("general")
  implicit val specific: B = B("specific")
class P { def x: Int = 1 }
trait Q; trait R extends Q
object Twin:
  object Both extends P with Q
  given both: (P & Q) = Both
  given other: P with { override def x = 2 }
object Parents:
  given pair: P() with R with {}
  given other: (P & Q) = new P with Q { override def x = 2 }
@main def run(): Unit =
  locally:
    import Givens.given
    println(summon[A].tag)
  locally:
    import Mixed.{given, *}
    println(summon[A].tag)
  locally:
    import Twin.given
    println(summon[P].x)
  locally:
    import Parents.given
    println(summon[P].x)
