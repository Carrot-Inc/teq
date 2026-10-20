trait Shape:
  def area: Int

object A:
  inline def square(n: Int): Shape = new Shape { def area = n * n }
  inline def choose(inline b: Boolean): Int = inline if b then LocalMacro.run else 0
  val first = square(3)
  val second = square(4)
  val used = choose(true)
  val third = square(5)
