object Z:
  val value: Int = M.next

@main def run(): Unit =
  println(A.f(Opt.B(1)))
  println(A.value)
  println(Z.value)
