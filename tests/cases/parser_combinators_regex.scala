//> using dep org.scala-lang.modules::scala-parser-combinators:2.4.0
// jars: scala-parser-combinators
// scala-parser-combinators' RegexParsers, whose `Parser(f)` makes `new Parser[T] { def apply(in: Input) }`
// in a method of the outer trait: the anonymous class's `apply` overrides `Function1.apply` of the
// parent's ancestors, which name the outer trait's `this` (`Parsers.this.Input`), seen from the class
// enclosing it (`RegexParsers.this`, `type Elem = Char`), as play's routes compiler parses its files.
import scala.util.parsing.combinator.*

object P extends RegexParsers:
  def word: Parser[String] = """[a-z]+""".r
  def pair: Parser[(String, String)] = word ~ ("=" ~> word) ^^ (p => (p._1, p._2))
  def pairs: Parser[List[(String, String)]] = repsep(pair, ",")

@main def run(): Unit =
  println(P.parseAll(P.pairs, "ab=cd,ef=gh").get)
  println(P.parseAll(P.pair, "ab=").successful)
