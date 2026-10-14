package core

object PgnDump:
  case class WithFlags(clocks: Boolean = true, moves: Boolean = true):
    def both = clocks && moves
