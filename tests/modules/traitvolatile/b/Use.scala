package tvb

import tva.*

class Worker extends Flags:
  @volatile var round: Int = 0

@main def run(): Unit =
  val w = Worker()
  w.stop = true
  w.round += 1
  println((w.stop, w.hit() + w.hit(), w.round, w.plain))
