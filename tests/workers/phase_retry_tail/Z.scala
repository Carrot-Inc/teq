object Z:
  val counted: Int = M.transparentNext
@main def run(): Unit =
  println(A.counted)
  println(Z.counted)
