var count = 0
def mk(): Fns = { count += 1; println("mk " + count); new Fns(count + 1) }
@main def run(): Unit =
  val g = timesOf(mk())
  println("made")
  println(g(1))
  println(g(10))
  println(count)
  val f = twiceOfLib
  println("made")
  println(f(21))
