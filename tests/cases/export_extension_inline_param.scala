class Ops(s: String) {
  inline def pad(inline n: Int, m: Int): Int = n + n + m
}
extension (s: String) {
  def ops: Ops = Ops(s)
  export ops.pad
}
@main def run(): Unit =
  println({ println("recv"); "ab" }.pad({ println("n"); 2 }, { println("m"); 3 }))
