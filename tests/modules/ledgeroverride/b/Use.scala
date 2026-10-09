package lob

import loa.*

object Use:
  def main(args: Array[String]): Unit =
    println(member(O))
    println(member(new C))
    println(outer(O))
