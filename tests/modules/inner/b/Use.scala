package inb

import ina.*

object Use:
  def main(args: Array[String]): Unit =
    val o = new Outer("t")
    println(o.inner(1).show + o.Helper.make(2).show)
    println(Registry.Defaults.entry.key + new Registry.Entry("k").key)
