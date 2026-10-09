package gfb

import gfa.Givens.given

object UsesGiven:
  def v: Int = summon[Int]

@main def run(): Unit = println(UsesGiven.v + 1)
