package p

object Checker:
  def kind(s: Plain): Int = s match
    case _: Marked => 1
    case _ => 0
