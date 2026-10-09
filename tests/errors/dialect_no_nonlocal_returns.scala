// teq: --dialect no-nonlocal-returns
// expect: a return from inside a function literal is not allowed under the dialect flag `no-nonlocal-returns`: a return from inside a function literal is an exception thrown through the literal and caught by the method, which every such method pays a try/catch for; use `boundary` and `boundary.break` in `scala.util`
def first(xs: List[Int]): Int =
  xs.foreach(x => if x > 1 then return x)
  0

def local(x: Int): Int =
  if x > 0 then return x
  -x

@main def main(): Unit =
  println(first(List(1, 2, 3)) + local(-1))
