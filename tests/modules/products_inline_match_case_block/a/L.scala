package ibk

object L:
  inline def first(x: Int): Int = x match
    case 1 => 1
    case _ => 0
