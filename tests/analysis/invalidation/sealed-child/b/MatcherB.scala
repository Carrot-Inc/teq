package scb

import sca.*

object MatcherB:
  def name(s: Shape): String = s match
    case Circle(_) => "c"
    case _ => "o"
