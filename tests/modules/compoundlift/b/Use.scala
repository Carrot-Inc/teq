package clb

import cla.*

object Use:
  def main(args: Array[String]): Unit =
    val s = new Shelf
    println(Ops.bumpField(2) + Ops.bumpField(3))
    println(Ops.bumpSetter(2) + Ops.bumpSetter(3))
    println(Ops.bumpMember(s, 2) + Ops.bumpMember(s, 3))
    println(Ops.bumpLazy(s, 2))
    println(Ops.plain(s, 1))
    println(new Meter().add(2) + new Meter().add(3))
    s.Bin.n += 1
    Tally.count += 1
    println((s.Bin.n, Tally.count))
