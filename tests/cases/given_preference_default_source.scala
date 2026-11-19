// From Scala 3.7 the more general of two givens wins (`CompareScheme.New`), and the given whose owner derives from the
// other's wins first: of a given of `A` and one of `B <: A` the general one; the derived object's general given over
// its trait's specific one; an alias of `A & B` against a given object of `A`, the object; a given object of two
// parents against an alias of a parent of theirs (`P & Q`, `R <: Q`), the alias. scalac 3.8.4 prints `general`,
// `general`, `2` and `2`.
class A(val tag: String); class B(tag: String) extends A(tag)
object Givens:
  given general: A = A("general")
  given specific: B = B("specific")
trait Low:
  given specific: B = B("specific")
object Derived extends Low:
  given general: A = A("general")
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
    import Derived.given
    println(summon[A].tag)
  locally:
    import Twin.given
    println(summon[P].x)
  locally:
    import Parents.given
    println(summon[P].x)
