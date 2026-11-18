// A condition without parentheses goes on over a line of the `if`'s own width, up to its `then`: no line end
// separates inside it. scalac prints 1.
def t(f: Int => Boolean, x: Int): Int =
  if f
  (x) then 1 else 2
@main def main(): Unit = println(t(_ > 0, 1))
