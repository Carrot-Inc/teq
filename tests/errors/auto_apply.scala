// expect: method f in object T must be called with () argument
// expect: method h in object T must be called with () argument
// expect: method g in object T does not take parameters
object T:
  def f() = println("f called")
  def g: Int = 1
  def h(): Int = 2

@main def run(): Unit =
  T.f
  val v = T.h
  println(T.g())
  println(v)
