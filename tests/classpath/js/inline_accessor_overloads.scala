// jars: fixtures
// targets: js interp jvm
// A jar's two inline methods of one name whose bodies reach two private overloads of one name through scalac's
// inline accessors, overloaded too (`inline$fix$overacc$AccAssertions$$checkImpl`): each body selects its accessor,
// and the accessor its member, by the signature its selection names (dotty's `Denotation.atSignature`), whatever
// the arguments' names.
import fix.overacc.AccAssertions

object Checks extends AccAssertions

@main def main(): Unit =
  println(Checks.check(1)(2))
  println(Checks.check(1, "clue")(2))
  println(Checks.check(expected = 1)(actual = 2))
  println(Checks.check(expected = "a", clue = 'c')(actual = 3.5))
