object Use:
  def a: Int = D.line
  def b: Int = D.line

@main def run(): Unit =
  println(Use.a)
  println(Use.b)
  println(summon[Show[D.P]].show(D.P(1)))
