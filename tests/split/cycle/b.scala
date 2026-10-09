package b

import a.*

trait Tagged:
  def tag: String = "tagged"

enum Mode extends Named:
  case Fast, Slow
  def name: String = s"mode-$toString"

object BConst:
  val value: String = "B+" + Color.Green.name

val bTop: String = "bTop sees " + Color.Red.greet

def other(n: Named): String = s"${n.greet}, ${Mode.Slow.greet}, ${AConst.value}"

class Pair(val color: Color, val mode: Mode) extends Named:
  def name: String = s"${color.name}&${mode.name}"
