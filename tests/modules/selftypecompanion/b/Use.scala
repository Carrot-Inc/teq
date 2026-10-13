import lib.Impl

@main def run(): Unit =
  val i = Impl(10)
  println(i.use("abc"))
  println(i.make("xy"))
