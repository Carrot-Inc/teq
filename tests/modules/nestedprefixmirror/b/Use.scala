package npmb

import npm.*
import scala.deriving.Mirror

object Use:
  def main(args: Array[String]): Unit =
    val o = new O(1)
    val m = summon[Mirror.SumOf[o.mid.T]]
    summon[m.MirroredElemTypes =:= Tuple1[o.C]]
    println(m.ordinal(new o.C(4)))
    val d = summon[Mirror.SumOf[o.mid.deep.U]]
    summon[d.MirroredElemTypes =:= Tuple1[o.Kids.D]]
    println(d.ordinal(new o.Kids.D(4)))
