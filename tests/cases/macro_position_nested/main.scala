@main def run(): Unit =
  println(Pos.here)
  println(Wrap.wrapper)
  println(Wrap.twice)
  println(Pos.gen)
  println(Wrap.genWrapped)
  println(Pos.file + " " + Wrap.fileWrapped)
