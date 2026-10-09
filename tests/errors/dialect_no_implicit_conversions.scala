// teq: --dialect no-implicit-conversions
// expect: the implicit conversion `toOps` is not allowed under the dialect flag `no-implicit-conversions`: a conversion is searched for at every selection of a member the receiver lacks and at every argument or result whose type does not conform; write an extension method
// expect: the implicit conversion `asText` is not allowed under the dialect flag `no-implicit-conversions`
import scala.language.implicitConversions

class Ops(x: Int):
  def twice: Int = x * 2

implicit def toOps(x: Int): Ops = new Ops(x)

class Text(val s: String)
given asText: Conversion[String, Text] = s => new Text(s)

@main def main(): Unit =
  println(5.twice)
  val t: Text = "a"
  println(t.s)
