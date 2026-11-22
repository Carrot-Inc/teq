package pmenum

import java.time.DayOfWeek

// A match on the same enum by its type alone: a pattern-match target too, since another source of the build
// (ByConstant) names one of its constants and the seal holds for the whole compilation.
object ByType:
  def any(d: DayOfWeek): Int = d match
    case x: DayOfWeek => 1
