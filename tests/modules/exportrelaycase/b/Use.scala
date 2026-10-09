package rcb

import rca.C

@main def run(): Unit =
  println(C.Box(7))
  val text: C.Text = "ok"
  val cell: C.Cell = new C.Cell(7)
  import C.*
  println(text.doubled + cell.value)
