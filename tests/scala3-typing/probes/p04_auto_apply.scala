object T:
  def f() = ()
  def g: Int = 1
  def h(): Int = 2
@main def run(): Unit =
  T.f
  println(T.g())
  val v = T.h
  println(v)
