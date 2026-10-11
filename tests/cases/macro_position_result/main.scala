@main def run(): Unit =
  var n = 1
  println(Lib.forwarded(n))
  println(Lib.quoted(n))
  println(M.loc(M.seven(n)))
