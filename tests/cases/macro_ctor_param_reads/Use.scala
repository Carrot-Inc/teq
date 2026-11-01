@main def run(): Unit =
  println(Mac.atCompile)
  val b = B(2)
  println(s"${b.saved} ${b.lam()} ${b.sam()} ${b.viaInline}")
