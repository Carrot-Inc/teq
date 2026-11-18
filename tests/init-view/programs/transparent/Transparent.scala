class T {
  transparent inline def pick(inline x: Int): Int = x * 2
  val a = pick(b)
  val b = 3
}

class U {
  val ok = { assert(n > 0); n }
  val n = 5
}
