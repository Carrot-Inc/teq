package sfb

import sfa.Forms

@main def run(): Unit =
  println(Forms.charLit("a|b"))
  println(Forms.charVar("a|b", 'b'))
  println(Forms.part("abc"))
  println(Forms.words("x,y"))
  println(Forms.sizes(List(1, 2)))
  println(Forms.sizesEq(List(1, 2)))
  println(Forms.flatOpt(Some(Some(3))))
  println(Forms.nullable(None) == null)
  println(Forms.scala(java.util.List.of("x")))
  println(Forms.range(3))
  println(Forms.vals(Map("a" -> 1)))
  println(Forms.dist(List(1, 1)))
  println(Forms.sorted(List("b", "a")))
  println(Forms.triple(Forms.makeTriple(1)))
