trait T:
  def show: String

@main def run(): Unit =
  val m = Map(1 -> "a", 2 -> "b")
  val ts = m.collect { case (k, v) if k > 0 => new T { def show = s"$k=$v" } }
  println(ts.map(_.show).toList.sorted.mkString(","))
