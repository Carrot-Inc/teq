package rounds

@main def run(): Unit =
  println(s"${Reads.line} ${Reads.fixed} ${Fixing.plain} ${Array(8, 2, 6).take(2).length} ${"p\nq\n".linesIterator.size}")
