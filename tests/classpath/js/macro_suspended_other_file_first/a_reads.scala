package rounds

object Reads:
  def at(using line: sourcecode.Line): Int = line.value
  val early: Int = at
  val fixed: Int = Fixing.level
