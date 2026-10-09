package acu

import acl.*

object Use:
  def main(args: Array[String]): Unit =
    val c = new Counter(1)
    println(c.tick(2))
    println(c.tick(3))
    println(Holder.peek)
    println(Holder.add(3) + Holder.add(4))
    var n = 0
    println(Holder.runTwice({ n += 1; n }))
