package pmtype

import java.time.DayOfWeek

// Matches on a Java enum of the JDK that name none of its constants, and a build that names none elsewhere: no
// pattern-match target, as scalac's class-file reader has not sealed the enum.
object ByTypeAlone:
  def any(d: DayOfWeek): Int = d match
    case x: DayOfWeek => 1

  def guarded(d: DayOfWeek): Int = d match
    case x if x.getValue > 3 => 1
    case _ => 0
