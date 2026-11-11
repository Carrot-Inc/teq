// jars: quote-patterns33-lib
// std: scala-library
// Read concrete and variable type patterns and term patterns from a Scala 3.3.7 library.
import quotepatterns33.*

object Main:
  def main(args: Array[String]): Unit =
    println(IsBool.test(true))
    println(IsBool.test(1))
    println(IsQualifiedBool.test(false))
    println(IsQualifiedBool.test("text"))
    println(BindOne.show(1))
    println(BindTwo.show(true, 1))
    println(TermVars.test(Some(1)))
    println(TermVars.test(1))
    println(TermPlain.test(args.length + 1))
    println(TermPlain.test(2))
    println(TermStable.test(None))
    println(TermStable.test(Some(1)))
    println(SingletonType.test[None.type])
    println(SingletonType.test[Int])
