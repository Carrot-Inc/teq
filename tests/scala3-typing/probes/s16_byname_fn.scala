object O:
  val a: Int => Int = x => x + 1
  val b: (=> Int) => Int = a
  def f(i: => Int): Int = 1
  def m[A](g: A => Int) = 2
  def n = m(f)
@main def run(): Unit = println(O.n)
