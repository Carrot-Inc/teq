package app

import facade.{format, Paths}

// the same binding as facade.join: one import statement serves both
@jsImport("node:path", "join")
def joinTwo(a: String, b: String): String

@main def main(): Unit =
  println(Paths.under("/srv", List("a", "b")))
  println(facade.join("x", "y", "z"))
  println(joinTwo("p", "q"))
  println(format("%s|%s", facade.separator, 1))
  val joiner: (String, String) => String = joinTwo
  println(List("1", "2", "3").foldLeft("0")(joiner))
