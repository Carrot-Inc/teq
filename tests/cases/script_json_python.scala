//> using platform jvm
//> using file ../../tools/script
// The library's `Json` against Python's `json`, whose bytes it promises (`tools/script/Json.scala`): a
// double as `repr` writes it (the shortest digits that read back, the nearest of them), keys sorted
// by code point, a repeated key's last value kept where its first stood, and what `json.loads`
// refuses refused: a leading zero, a control character in a string, a missing end. Its expectation is
// scalac's run, checked against Python's output for the same values.
object Main:
  def main(args: Array[String]): Unit =
    val doubles = List(Double.MinPositiveValue, 3 * Double.MinPositiveValue, 2.2250738585072014e-308, 1e-5, 1e-4, 0.1, 0.1 + 0.2,
      1.0 / 3, 2.5, 100.0, 1e15, 1e16, 1e22, 1e23, 123456789.123, 9007199254740993.0, 5e-324 * 1e10, 1.5e300,
      Double.MaxValue, -0.0, Math.pow(2, -1022) * 3, Math.pow(2, 60), Double.NaN, Double.PositiveInfinity, Double.NegativeInfinity)
    for d <- doubles do println(Json.write(Json.num(d)))
    val repeated = Json.parse("{\"a\": 1, \"b\": 2, \"a\": 3}")
    println("repeated=" + Json.write(repeated) + " " + repeated("a").int)
    println("sorted=" + Json.write(Json.obj("\uE000" -> Json.num(1), "\uD83D\uDE00" -> Json.num(2), "b" -> Json.num(3), "B" -> Json.num(4)), sortKeys = true))
    for input <- List("01", "-01", "0", "-0", "0.5", "1e5", "1E+5", "-", "1.", ".5", "\"line\nline\"", "\"tab\tin\"", "\"esc\\u0001\"",
        "{", "[1,", "[1,]", "NaN", "-Infinity", "[Infinity]", "nan", "\"\\x\"", "  [1, 2]  ", "[1] x") do
      try println("parse " + input.replace("\n", "\\n").replace("\t", "\\t") + " = " + Json.write(Json.parse(input)))
      catch case e: Json.ParseError => println("parse " + input.replace("\n", "\\n").replace("\t", "\\t") + " = rejected")
