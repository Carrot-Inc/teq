object O:
  def f(x: Missing): Int = 1
  val v: AlsoMissing = ???
enum Outer:
  case Foo(u: Unavailable)
@main def run(): Unit = println(1)
