@main def run(): Unit =
  println(Inspect.result)
  println(CopiedChild.result(Create.box("value")))
  println(TypePaths.result)
