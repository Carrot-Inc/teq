package pib

import pia.P

@main def run(): Unit =
  println(P.use(P.id))
  println(P.both(P.id))
  println(P.inst(P.id)(7))
  println(P.instOnce)
  println(P.label(P.run))
