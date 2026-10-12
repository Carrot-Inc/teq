// A `using` before a parenthesis begins a using argument list, whatever follows it: a function
// literal with its parameters in parentheses is the argument (Parsers.parArgumentExprs).
def twice(n: Int)(using f: Int => Int): Int = f(f(n))

@main def main(): Unit =
  println(twice(3)(using (x: Int) => x + 1))
  println(twice(3)(using (x) => x * 2))
  println(twice(3)(using x => x - 1))
  println(twice(3)(using (_ + 10)))
