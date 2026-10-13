// A local `inline given` is an inline method (dotty's `Parsers.givenDef`), found by the search as
// any given and expanded after it: one that reads itself exceeds the inlines' limit there, as
// scalac reports it, rather than failing as a candidate.
// expect: 8:11: error: Maximal number of successive inlines (32) exceeded,
// expect: 1 error found
@main def run(): Unit =
  inline given x: Int = x
  println(summon[Int])
