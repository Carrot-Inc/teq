@main def run(): Unit =
  println(Marked.marked(41 + 1))
  println(Marked.marked("kept"))
  println(Marked.parameters)
  println(Marked.owned)
  println(Marked.synthetic)
