// A plain inline call as a tuple's index: the index reads its expansion.
object M:
  inline def idx: Int = 1
  inline def one: 1 = 1

@main def main(): Unit =
  println((42, "yes")(M.idx))
  println((42, "too")(M.one))
