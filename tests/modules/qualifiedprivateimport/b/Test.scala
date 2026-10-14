package p

import p.internal.Repo.*

object Test:
  def main(args: Array[String]): Unit =
    println(use(Side.From))
