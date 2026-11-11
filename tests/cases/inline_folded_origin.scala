object Arithmetic:
  inline def calculate(value: Int): Int =
    val first = value + 3
    val second = first * 5
    val third = second + 7
    val fourth = third * 11
    fourth + 13
@main def run(): Unit =
  println(Arithmetic.calculate(2))
  println(Arithmetic.calculate(4))
