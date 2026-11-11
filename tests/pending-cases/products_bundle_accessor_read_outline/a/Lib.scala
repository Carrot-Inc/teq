package aroa

class C:
  inline def add(n: Int): Int = { C.total += n; C.total }

object C:
  private var total = 0
