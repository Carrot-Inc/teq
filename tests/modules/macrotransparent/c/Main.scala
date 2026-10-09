package mtc

import mtb.B

@main def run(): Unit =
  println(B.picked + 1)
  println(B.word.length)
  B.first.run()
  B.second.run()
  mta.T.runner("third").run()
