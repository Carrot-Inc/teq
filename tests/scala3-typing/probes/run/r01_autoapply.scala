object T:
  def f() = println("f called")
  def h(): Int = { println("h called"); 2 }
@main def run(): Unit =
  T.f
  val v = T.h
  println(v)
