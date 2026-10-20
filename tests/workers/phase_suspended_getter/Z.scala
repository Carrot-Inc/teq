object Z:
  val c = A.C(1)
  val value: Int = M.next

@main def run(): Unit =
  println(Z.c.b)
  println(A.value)
  println(Z.value)
