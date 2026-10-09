object O extends X:
  def o: Int = 3

class X extends Y:
  def x: Int = 2

@main def run(): Unit =
  println(O.o + O.x + O.y + O.z)
