package vab

import vaa.*

class Worker extends Flags:
  @V var round: Int = 0

@main def run(): Unit =
  val w = Worker()
  w.stop = true
  w.round += 1
  println((w.stop, w.round, w.plain))
