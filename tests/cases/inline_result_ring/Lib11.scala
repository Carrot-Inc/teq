object Lib11:
  inline def outer0(x: Int) = Lib12.inner0(x) + 12
  inline def inner0(x: Int) = x * 2
  inline def outer1(x: Int) = Lib10.inner1(x) + 13
  inline def inner1(x: Int) = x * 3
  inline def outer2(x: Int) = Lib12.inner2(x) + 14
  inline def inner2(x: Int) = x * 4
  def total(x: Int): Int = outer0(x) + outer1(x) + outer2(x)
