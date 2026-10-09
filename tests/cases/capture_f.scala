// `tupled` and `curried` hold the function in a local, whose initialiser reads the definition
// named as it is.
def `f$0`(): (Int, Int) => Int = (a, b) => a * 10 + b

@main def main(): Unit =
  println(`f$0`().tupled((1, 2)))
  println(`f$0`().curried(1)(2))
