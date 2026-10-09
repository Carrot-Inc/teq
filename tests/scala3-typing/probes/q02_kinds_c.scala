object T:
  def f[F[_]] = 1
  def k = f[Map]
  val l: List[List] = ???
@main def run(): Unit = println(1)
