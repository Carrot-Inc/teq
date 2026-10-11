@main def run(): Unit =
  println(Q.curried(x => y => x + y))
  println(Q.folded(x => x + 3))
  println(Q.runCurried(x => y => x * 10 + y))
  println(Q.runFolded(x => x * 2))
  println(Q.runEffect(x => { println("b"); x + x }))
