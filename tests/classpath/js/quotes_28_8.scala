// jars: fixtures
// A macro jar compiled by Scala 3.8 (TASTy 28.8), which pickles a quote as `'{ e }.apply(q)`: the
// quote of the macro's argument and a quote with a splice in it.
import fix.q38.Quotes38

object Main:
  def main(args: Array[String]): Unit =
    println(Quotes38.twice(21))
    println(Quotes38.described(7))
