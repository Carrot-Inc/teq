package api

import game.PgnDump.WithFlags

object Use:
  def main(args: Array[String]): Unit =
    val f: WithFlags = WithFlags()
    val g = game.PgnDump.WithFlags(moves = false)
    println(s"${f.both} ${g.both}")
