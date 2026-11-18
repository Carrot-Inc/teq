package rotb

@main def run(): Unit =
  val narrow: Option[rota.Step] = rota.Steps.saved
  val narrowVar: Option[rota.Step] = rota.Steps.savedVar
  val narrowDef: Option[rota.Step] = rota.Steps.savedDef
  println(List(narrow, narrowVar, narrowDef).flatten.mkString(" "))
