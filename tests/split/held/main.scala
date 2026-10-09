package app

@main def run(): Unit =
  println(logic.isSelection(ui.first))
  println(logic.index(ui.first))
  println(logic.name(ui.Mark.Red))
  println(logic.isRed(ui.Mark.Green))
  println(logic.label("a"))
