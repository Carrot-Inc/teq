package ovb

import ova.*

object Use:
  def main(args: Array[String]): Unit =
    val p = new Printer
    println(p.print(1) + p.print("a") + p.print(1, 2))
    val c: Container[Int] = new Container(List(1, 2))
    println(s"${c.first}${c.mapped(_.toString).first}")
