object CounterDomain:
  opaque type Tick = Long
  extension (x: Tick) def until(y: Tick): Long = y - x

import CounterDomain.*

@main def test(): Unit =
  val indices = (0 until 4).toList
  assert(indices == List(0, 1, 2, 3))
  println("ok")
