package jb

import ja.*

object Use:
  def main(args: Array[String]): Unit =
    val id = Ids.fixed
    println(Ids.text(id) + " " + id.getLeastSignificantBits)
