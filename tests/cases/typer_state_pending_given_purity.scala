// A plain inline given still pending is its expansion, not a stable path: an eta-expansion lifts
// it out of the function, once (`next()` runs when the function is made), and named arguments
// reordered keep it first.
var count = 0
def next(): Int =
  count += 1
  count
def step(n: Int): Int =
  println(n)
  n

object A:
  inline given tick: Int = next()
  def f(using i: Int)(a: Int): Int = i + a
object B:
  inline given tock: Int = step(1)
  def f(using i: Int)(a: Int, b: Int): Int = i + a + b

def eta(): Unit =
  import A.given
  val g: Int => Int = A.f
  println(count)
  println(g(10))
  println(g(20))
  println(count)

def named(): Unit =
  import B.given
  println(B.f(b = step(2), a = step(3)))

@main def main(): Unit =
  eta()
  named()
