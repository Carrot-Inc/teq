// An eta-expansion's parameter meets a definition named as it is, which an argument of the
// method's earlier parameter list reads: a plain parameter, a pair untupled, a by-name one.
def `eta$0`(): Int = 7

def m(a: Int)(b: Int): Int = a * 10 + b
def key(a: Int)(b: Int, c: Int): Int = a * 100 + b * 10 + c
def twice(a: Int)(x: => Int): Int = a * 100 + x + x

@main def main(): Unit =
  val f: Int => Int = m(`eta$0`())
  val g: ((Int, Int)) => Int = key(`eta$0`())
  val h: (=> Int) => Int = twice(`eta$0`())
  println(f(1))
  println(g((1, 2)))
  println(h(1))
