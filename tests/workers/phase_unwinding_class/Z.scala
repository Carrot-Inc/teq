@main def run(): Unit =
  println(S.Live().send(1))
  println(summon[Show[S.Cat]].name)
