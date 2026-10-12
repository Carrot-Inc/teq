package app
package sub

def f: Option[Int] = 1.some
def g: FiniteDuration = 10.days
def h: Fu[Int] = List(1)

@main def run(): Unit =
  println(f)
  println(g)
  println(h)
  println(21.twice)
  println(List(true, false).sorted)
