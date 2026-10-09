given Show[Int] with
  def show(a: Int) = s"int $a"

@main def run(): Unit =
  println(Macros.showIt(3))
  println(Macros.showIt(4))
