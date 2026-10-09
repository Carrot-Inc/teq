package pdb

import pda.PolyDep

object UsePolyDep:
  val s: String = "poly"
  val same: s.type = PolyDep.f[String](s)
  val kept: s.type = PolyDep.keep(PolyDep.f)[String](s)
  def main(args: Array[String]): Unit = println(same + kept)
