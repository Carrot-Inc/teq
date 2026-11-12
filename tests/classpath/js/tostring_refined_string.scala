// jars: refined
// teq: --wtostring-interpolated --werror
//> using dep eu.timepit::refined:0.11.4
//> using options -Wtostring-interpolated -Werror
// refined's `NonEmptyString` (an opaque alias with a `String` upper bound) interpolated: no use of `toString`, as
// `tests/cases/tostring_opaque_string` has it without the library. scalac 3.8.4 compiles this under
// `-Wtostring-interpolated -Werror` and prints `($3 fee)`.
import eu.timepit.refined.types.string.NonEmptyString

final case class Amount(value: BigDecimal)
extension (a: Amount) def shown: NonEmptyString = NonEmptyString.unsafeFrom("$" + a.value.toString)

@main def run(): Unit =
  val a = Amount(BigDecimal(3))
  println(s"(${a.shown} fee)")
