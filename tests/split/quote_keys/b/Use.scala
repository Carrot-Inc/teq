package keys

object A:
  def a: Show = Macros.make("a")

@main def run(): Unit =
  println(A.a.show)
