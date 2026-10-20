trait Shape:
  def area: Int

object A:
  inline def choose(inline b: Boolean): Int = inline if b then LocalMacro.run else 0
  inline def square(n: Int): Shape = new Shape { def area = n * n }
  inline def twice(n: Int): Int = n + n
  val used = choose(true)
  val first = square(3)
  val second = square(twice(2))
  def third(k: Int): Shape = square(k + twice(k))
