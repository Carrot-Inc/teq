// The proxy an inline method binds for an argument with effects keeps the parameter's name,
// here `Math`, which the multiplication of two `Int`s reads as JavaScript's `Math.imul`.
inline def times(Math: Int, x: Int): Int = Math * x

def three(): Int =
  println("three")
  3

@main def main(): Unit =
  println(times(three(), 5))
