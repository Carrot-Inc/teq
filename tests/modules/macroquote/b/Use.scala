package mqb

import mqa.Mac

@main def run(): Unit =
  println(Mac.twice(21))
  println(Mac.pair(1, "x"))
  println(Mac.nested(4))
  println(Mac.typeName[Option[Int]])
  Mac.runner("one").run()
  Mac.runner("two").run()
  println(Mac.boxed(5))
  println(Mac.boxed(6))
