package icm

import icb.Made

@main def later(): Unit =
  println(Made.touch)
  println(List(Made.anon, Made.local, Made.nested, Made.own, Made.ownLocal, Made.withUnused, Made.picked).map(_.dead(0)) :+ Made.plain.dead)
