package rounds

@main def run(): Unit =
  println(s"${Reads.counted} ${Reads.fixed} ${Fixing.plain} ${Array(8, 2, 6).take(2).length}")
