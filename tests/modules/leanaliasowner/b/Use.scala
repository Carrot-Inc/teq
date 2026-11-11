package lab

import lao.Lib

@main def run(): Unit =
  println(Lib.pair((1, 2)))
  println(Lib.empty)
  println(Lib.named[String])
  println(Lib.test[String]("s"))
  println(Lib.test[String](1))
