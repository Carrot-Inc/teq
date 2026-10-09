object A:
  println("A init")
  val x = { println("x init"); 1 }
  def m(i: Int) = i + x
  lazy val l = { println("l init"); 2 }
object B:
  println("B init")
  export A.{x, m, l}
  println("B end")
@main def main(): Unit =
  println("main")
  println(B.m(10))
  println(B.x)
  println(B.l)
  println(Facade.n)
  println(Facade.m(1))
object Facade:
  export A.{x as n, m}
