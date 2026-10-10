package iqb

import iqa.Q

@main def run(): Unit =
  println(Q.value(3, x => x + 1))
  println(Q.code(1 + 2, x => x + 1))
  println(Q.shape(x => x + 1))
