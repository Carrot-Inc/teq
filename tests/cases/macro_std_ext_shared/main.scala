package shared

@main def run(): Unit =
  println(Mac.tidy("  At Compile Time "))
  val s = "  At Run Time "
  val boxed: java.lang.Integer = s.length
  println("[" + s.trim().toLowerCase(java.util.Locale.ROOT) + boxed + "]")
