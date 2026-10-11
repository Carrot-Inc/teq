package dplanuse

import dplan.Api
import dplan.Api.++:

@main def main(): Unit =
  Api.usingClause()
  Api.byName()
  Api.repeated()
  Api.inlineClause()
  Api.inlineUsing()
  Api.rightAssoc()
  Api.inlineRightAssoc()
  println(("x" ++: "y")())
  Api.inlineRepeated()
  Api.ctor()
  Api.f()(using Api.mark("u"))()
  println()
