// teq: --threads 4
@main def run(): Unit =
  val t: Int = M.total
  println(t)
  println(Kept.items.mkString(","))
