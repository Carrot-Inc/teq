@main def run(): Unit =
  val k = 3
  println(M.d(k))
  println(M.d(k + 1))
  println(M.d(k: Any))
  println(M.d(("ab": String).length))
  println(M.d(List(1, k)))
  println(M.d(List[Any](1, "a")))
  val box: Box = new Box { type T = Int }
  println(M.d({ val c: Box = box; List.empty[c.T] }))
  println(M.d((z: Int) => z + 1))
  println(M.d((z: Int) => 5))
  println(M.directions())
