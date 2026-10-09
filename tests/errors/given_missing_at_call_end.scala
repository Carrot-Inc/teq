// expect: 10:18: error: no given instance of type String was found for parameter s
// expect: 12:7: error: no given instance of type String was found for parameter s
// expect: 13:20: error: no given instance of type String was found for parameter s
// A using clause no given fills is reported at the end of what the call applies so far (dotty's
// `adaptNoArgsImplicitMethod`, which asks for the argument at `tree.span.endPos`): the call's end,
// over several lines too, or the end of the lists before a clause a written list follows.
def need(a: Int)(using s: String): Int = a + s.length
def middle(a: Int)(using s: String)(b: Int): Int = a + b + s.length
@main def run(): Unit =
  println(need(1))
  println(need(1 +
    2))
  println(middle(1)(2))
