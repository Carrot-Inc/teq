import Macros.*

case class P(a: Int, b: String)

@main def run(): Unit =
  println(describe[P](p => p.b))
  println(describe[Int](n => n + 1))
  println(describe2[Int, String]((n, s) => s * n))
