// jars: quote-patterns33-lib
// A library's macro called before a macro of the run in one file. The inlining phase's first round
// runs the library's macro, whose bodies are typed for that run, and gives the file up at the
// macro of the run; the file is typed again and the second round runs the library's macro over
// the bodies the first round typed (tests/support/quote_patterns33_lib.scala, whose type patterns
// hand their `Type` on as a given).
import quotepatterns33.*

@main def run(): Unit =
  println(BindOne.show(1))
  println(BindTwo.show(true, 1))
  println(Macros.answer)
  println(BindOne.show(2.5))
  println(IsBool.test(false))
