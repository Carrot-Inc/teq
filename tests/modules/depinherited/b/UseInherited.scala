package dib

import dia.*

object UseInherited:
  def result(d: Derived, c: Ctx): c.T = d.inferred(c)
  def main(args: Array[String]): Unit =
    val c = new Ctx { type T = Int; val t = 7 }
    val n: Int = result(Impl, c)
    println(n)

object Impl extends Derived:
  def value: (c: Ctx) => c.T = c => c.t
