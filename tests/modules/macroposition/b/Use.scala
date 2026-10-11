package mpb

import mpa.{Lib, M}

@main def run(): Unit =
  var n = 1
  println(Lib.forwarded(n))
  println(M.loc(M.seven(n)))
