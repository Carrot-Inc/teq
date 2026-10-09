@main def run(): Unit =
  val m = Map(1 -> "a", 2 -> "b")
  val ts = m.collect { case (k, v) if k > 0 => Counter.next() }
  println(ts.toList.sorted.mkString(","))
  println(Counter.next())
