// expect: expected a definition

// A blank line ends the signature, so the second clause is not part of the def.
def blankLine(a: Int)

  (b: Int): Int = a + b

@main def run(): Unit =
  println(blankLine(1)(2))
