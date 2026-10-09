package dfc

import dfb.ChainMid

object ChainEnd:
  def main(args: Array[String]): Unit =
    println(ChainMid.message)
    println(ChainMid.passed + ChainMid.precise)
    println(dfa.DepFn.inspect(List("end"))(xs => summon[String] + xs.size))
