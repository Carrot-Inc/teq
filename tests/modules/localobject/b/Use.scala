package lob

import loa.Loc

@main def run(): Unit =
  println(Loc.pairs(List(1, (2, 3))))
  println(Loc.counter())
  println(Loc.pairsInline(List((4, 5))))
