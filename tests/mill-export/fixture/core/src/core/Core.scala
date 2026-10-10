package core

object Core:
  def line(using l: sourcecode.Line): Int = l.value
  val name: String = "core"
