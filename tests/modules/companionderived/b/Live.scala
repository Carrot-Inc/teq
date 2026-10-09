package cdb

import cda.Codec
import cda.FlagModels.Flags

final class Live(names: List[String]):
  def make(on: Boolean): Flags = mk(on, names)

  private def mk(on: Boolean, ns: List[String]) =
    Flags(
      on = on,
      names = { println("names"); ns }
    )

@main def run(): Unit =
  val live = Live(List("a"))
  println(live.make(true))
  println(summon[Codec[Flags]].name)
