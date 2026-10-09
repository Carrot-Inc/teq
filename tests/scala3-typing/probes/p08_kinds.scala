object T:
  class C[F[_]]
  val x: C[C] = ???
  def f[F[_]] = 1
  def g = f[Int]
  def h = f[List]
  def k = f[Map]
@main def run(): Unit = println(1)
