// teq: --module-per-file ui
package app

import ui.*

@main def run(): Unit =
  println(Box(3).sized)
  println(Tag("t").describe)
  println(LeftConst.text)
  println(leftTop)
  println(rightTop)
  println(Tone.values.toList)
  println(Flavor.valueOf("Sour").ordinal)
  val shapes: List[Shape] = List(Flavor.Sweet, Tag("x"))
  shapes.foreach(s => println(s.describe))
  val sized: List[Sized] = List(Tone.Loud, Box(1))
  sized.foreach(s => println(s.sized))
