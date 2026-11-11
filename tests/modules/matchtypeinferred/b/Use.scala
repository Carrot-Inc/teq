package mtu

import mti.Api

object Use:
  def main(args: Array[String]): Unit =
    val i: Int = Api.head(List(1, 2))
    val s: String = Api.head(Option("x"))
    println(s + i)
