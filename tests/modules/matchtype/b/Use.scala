package mtb

import mta.*

object Use:
  def main(args: Array[String]): Unit =
    val c: Char = Elems.first("abc")
    val i: Int = Elems.first(List(1, 2))
    println(c.toString + i)
