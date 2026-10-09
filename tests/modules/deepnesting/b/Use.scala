package dnb

import dna.Ladder

@main def run(): Unit =
  println(Ladder.rank(17))
  println(Ladder.rank(99))
  println(Ladder.nested(1))
