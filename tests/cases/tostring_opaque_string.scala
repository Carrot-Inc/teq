// teq: --wtostring-interpolated --werror
//> using options -Wtostring-interpolated -Werror
// An interpolated value whose type is an opaque alias bounded by `String` (refined's `Refined`, written here) is
// no use of `toString`: dotty's `StringInterpolatorOpt.lintToString` runs after `ElimOpaque`, where the alias is a
// `String`. scalac 3.8.4 compiles this under `-Wtostring-interpolated -Werror` and prints `($3 fee)`.
object refined:
  opaque type Refined[+T, P] <: T = T
  object Refined:
    def unsafeApply[T, P](t: T): Refined[T, P] = t
  trait NonEmpty
  type NonEmptyString = Refined[String, NonEmpty]

import refined.*

final case class Amount(value: BigDecimal)
extension (a: Amount) def shown: NonEmptyString = Refined.unsafeApply("$" + a.value.toString)

@main def run(): Unit =
  val a = Amount(BigDecimal(3))
  println(s"(${a.shown} fee)")
