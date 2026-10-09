object T:
  def f[F[_]] = 1
  def g = f[Int]
@main def run(): Unit = println(1)
