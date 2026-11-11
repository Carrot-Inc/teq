// An argument the compiler evaluates before binding its parameter (tests/fold.sh) is shown by `codeOf` as written, as
// dotty substitutes an inline parameter's argument as its tree: alone, twice in an operation, passed on to another
// inline method; a by-value parameter shows its proxy. scalac prints the lines of the .expected file.
import scala.compiletime.codeOf
inline def shown(inline n: Int): String = codeOf(n)
inline def shownSum(inline n: Int): String = codeOf(n + n)
inline def shownValue(n: Int): String = codeOf(n)
inline def passOn(inline n: Int): String = shown(n)
@main def run(): Unit =
  println(shown("ab".length + 1))
  println(shown("ab".length + 1) == "3")
  println(shownSum("abc".length))
  println(shownValue("abc".length).startsWith("n$proxy"))
  println(passOn("ab".length + 1))
