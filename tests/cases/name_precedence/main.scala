// The empty package is visible to its own files only: the standard library keeps its List.
class List[T](val x: T)
object Result:
  def n = 1

@main def run(): Unit =
  println(new List(7).x)
  println(scala.List(1, 2).map(_ + Result.n))
  app.Main.run()
