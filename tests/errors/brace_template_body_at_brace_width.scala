trait T:
  def m: Int
object P:
  def run(): Int =
    val r = { new T:
      def m = 1
    }
    r.m
@main def main(): Unit = println(P.run())
// Braces with a template opened on their own line: the braces take the width of their first line break's line, the
// template's body line, so that line is no wider than its region and opens no indented body. scalac: "indented
// definitions expected, def found" at the body's first token.
// expect: 6:7: error:
