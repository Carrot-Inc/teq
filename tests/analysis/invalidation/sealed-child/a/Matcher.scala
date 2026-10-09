package sca

object Matcher:
  def name(s: Shape): String = s match
    case Circle(_) => "c"
    case Square(_) => "s"
    case _ => "o"
