package app

import lib.Ops

@main def run(): Unit =
  val o = new Ops(2)
  println(s"${o.tagged} ${o.boxes(2).map(_.sum).sum} ${o.same} ${o.differ}")
  println(s"${o.fresh} ${o.cells} ${o.nested} ${o.projected} ${o.inlined}")
  val made = Array(new o.nums.Box(1), new o.nums.Box(2))
  println(s"${made.map(_.sum).sum} ${Array.empty[o.nums.Box].length} ${new o.nums.Money(1) == new o.nums.Money(1)}")
  val b: o.nums.Box = new o.nums.Box(1)
  println(b match { case _: o.nums.Box => "box" })
