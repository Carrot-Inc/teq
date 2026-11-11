package amoa

class Counter(start: Int):
  private var count = start
  private def bump(n: Int): Int = { count += n; count }
  inline def tick(n: Int): Int = bump(n) + count
