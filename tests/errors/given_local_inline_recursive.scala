// A local `inline given` is an inline method (dotty's `Parsers.givenDef`), found by the search as any given and
// expanded after it: one that reads itself exceeds the inlines' limit there, as scalac reports it, rather than failing
// as a candidate, at the end of the call it is the argument of (`Typer.implicitArgs`' `endPos`; scalac's 8:21 from 0).
// expect: 8:22: error: Maximal number of successive inlines (32) exceeded,
// expect: 1 error found
@main def run(): Unit =
  inline given x: Int = x
  println(summon[Int])
