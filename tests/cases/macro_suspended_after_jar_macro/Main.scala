// jars: quote-patterns33-lib
// A library's macro called before
// a macro of the run in one file
// (tests/support/quote_patterns33_lib.scala,
// whose type patterns hand
// their `Type` on as a given).
import quotepatterns33.*

@main def run(): Unit =
  println(BindOne.show(1))
  println(BindTwo.show(true, 1))
  println(Macros.answer)
  println(BindOne.show(2.5))
  println(IsBool.test(false))
