package dmb

import scala.deriving.Mirror
import dma.Point

// A mirror of the upstream's class that only the downstream summons: teq synthesizes it where
// the class's file stands, which the whole build has among its sources and the split build
// does not (scalac's mirror is the companion, whichever module asks).
@main def run(): Unit =
  val m = summon[Mirror.ProductOf[Point]]
  println(m.fromProduct((1, 2)))
