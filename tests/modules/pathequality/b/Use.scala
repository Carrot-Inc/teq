package app

import lib.Ops

@main def run(): Unit =
  val o = new Ops(2)
  println(s"${o.same} ${o.differ} ${o.inlined} ${o.expanded}")
  println(s"${new o.nums.Money(1) == new o.nums.Money(1)} ${new o.nums.Money(1) != new o.nums.Money(1)}")
