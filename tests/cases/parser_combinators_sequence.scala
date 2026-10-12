//> using dep org.scala-lang.modules::scala-parser-combinators:2.4.0
// jars: scala-parser-combinators
// scala-parser-combinators' sequencing, whose `Success.flatMapWithNext` makes `new Success[U](res, next) {
// override val lastFailure = .. }` in the body of the trait's case class: the anonymous class's parent is the
// class `Success` itself through the trait's `this`, not a lambda over it (the jar's unapplied `TypeRef`).
import scala.util.parsing.combinator.*

object Q extends RegexParsers:
  def word: Parser[String] = """[a-z]+""".r
  def assign: Parser[String] = word ~ ("=" ~> word) ^^ (p => s"${p._1}->${p._2}")

@main def run(): Unit =
  println(Q.parseAll(Q.assign, "ab=cd").get)
  println(Q.parseAll(Q.assign, "ab=").successful)
