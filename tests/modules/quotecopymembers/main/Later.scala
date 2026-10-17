package emd

import ema.EnumMirror
import emb.Made
import emb.Models.Color

@main def later(): Unit =
  println(Made.touch)
  val m = summon[EnumMirror[Color]]
  println(m.ordinal(Color.Green))
  println(List(Made.anon, Made.local, Made.solo, Made.given_, Made.nested, Made.unused, Made.holder).map(_.dead(0)))
