package pmenum

import java.time.DayOfWeek

// A match on a Java enum of the JDK, read from its class file, naming one of its constants: the enum is a
// pattern-match target (`PatMatTarget`), as scalac's class-file reader seals a Java enum once a constant's info
// is completed.
object ByConstant:
  def weekday(d: DayOfWeek): Int = d match
    case DayOfWeek.MONDAY => 1
    case _ => 0
