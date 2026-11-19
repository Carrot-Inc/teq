// A Scala 2 implicit is compared the old way on its own side (dotty's `isAsGoodValueType`, `alt1IsImplicit`): a given
// of `A` and an implicit of `B <: A` each win their own side from 3.7 on, an ambiguity (scalac 3.8.4: E172 at the
// search).
class A(val tag: String); class B(tag: String) extends A(tag)
object O:
  given general: A = A("general")
  implicit val specific: B = B("specific")
@main def run(): Unit =
  import O.{given, *}
  println(summon[A].tag)
// expect: 10:20: error: ambiguous given instances for A: general, specific
