// Enumerators that begin on the `for`'s own line, without brackets, go on over the lines after it, at the
// `for`'s width or further in: generators, a guard, a definition, up to the `yield`. scalac prints the same lines.
def sameWidth(xs: List[Int]): List[Int] =
  for x <- xs
  y <- List(x, x * 10)
  yield y

def wider(xs: List[Int]): List[Int] =
  for x <- xs
      y <- List(x, x * 10)
  yield y

def guardAndDefinition(xs: List[Int]): List[Int] =
  for x <- xs
      if x > 1
      z = x * 2
  yield z

@main def main(): Unit =
  println(sameWidth(List(1, 2)))
  println(wider(List(1, 2)))
  println(guardAndDefinition(List(1, 2, 3)))
