trait Slot:
  var x: Int
  def reset(): String = Probe.assignment { x = 0 }

class Cell:
  var n = 0

@main def runAbstractVarAssign(): Unit =
  val cell = Cell()
  val t: Slot = new Slot { def x = cell.n; def x_=(v: Int) = cell.n = v * 2 }
  println(Probe.assignment { t.x = 7 })
  println(cell.n)
  println(t.reset())
  Probe.setTo(t.x, 5)
  println(cell.n)
  val c = Cell()
  println(Probe.assignment { c.n = 3 })
