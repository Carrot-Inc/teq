package upc

import upa.*

object Client:
  def main(args: Array[String]): Unit =
    println(L.applied(new ZU1))
    println(L.aliased(new ZU1))
