package rounds

@main def run(): Unit =
  println(s"${Reads.early} ${Reads.fixed} ${Array(8, 2, 6).take(2).length} ${"p\nq\n".linesIterator.size}")
