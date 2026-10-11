package sam

import saa.*
import sab.*

@main def run(): Unit =
  println(Model.rops.name)
  println(Model.me eq Model)
  val g = new Graph
  println(g.rops.name)
  val r: ROps[g.type] = g.rops
  println(r.name)
