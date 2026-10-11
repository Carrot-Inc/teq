package use

import extractorclash.*

object Use:
  def main(args: Array[String]): Unit =
    println(C(7) match { case C(n) => n })
    println(C("abcd") match { case C(n) => n; case _ => -1 })
    println(Box(23) match { case Box(x) => x })
    println(Inh(2) match { case Inh(n) => n; case _ => -1 })
