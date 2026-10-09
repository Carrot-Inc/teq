class C(x: Int):
  def bump(): Unit = x = 2
case class D(x: Int)
@main def run(): Unit =
  val v = 1
  v = 2
  val d = D(1)
  d.x = 3
  println(d)
