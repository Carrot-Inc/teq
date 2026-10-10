package harness

object Harness:
  def line(using l: sourcecode.Line): Int = l.value
  val name: String = "harness"
