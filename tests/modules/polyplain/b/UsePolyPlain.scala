package ppb

import ppa.PolyPlain

object UsePolyPlain:
  def main(args: Array[String]): Unit = println(PolyPlain.keep([A] => (a: A, b: A) => a)[String]("first", "second"))
