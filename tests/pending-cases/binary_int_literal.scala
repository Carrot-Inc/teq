// Binary integer literals (Scala 3.5's `0b`/`0B`, digit separators among them): scalac 3.8.4 reads each as a number,
// teq's lexer refuses each ("5 errors found").
@main def run(): Unit =
  println(0b101)
  println(0B11L)
  println(0b1111_0000)
  println(-0b1)
  println(0b11111111111111111111111111111111)
