package umb

import uma.*
import uma.Meter.seen

object Use:
  def main(args: Array[String]): Unit =
    println(seen(Board.expanded) + " " + seen(Board.chosen(true)) + " " + seen(Board.plain))
    println(seen(Kept.once) + " " + seen(Kept.once) + " " + seen(Kept.mark) + " " + seen(Kept.mark))
    println(seen(Kept.slot))
    Kept.slot = Meter.tick("set")
    println(seen(Kept.slot))
    val f = new Fields
    println(seen(f.first) + " " + f.second + " " + seen(f.third))
    val typed: Source[Unit] = f
    val wide: Source[Any] = f.asInstanceOf[Source[Any]]
    println(seen(typed.next) + " " + seen(wide.next))
    println(Meter.ticks)
