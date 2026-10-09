package roots.data

/** A product of its own: `productElement` is called through the runtime's template. */
class Pair(val a: Int, val b: Int) extends Product:
  def canEqual(that: Any): Boolean = that.isInstanceOf[Pair]
  def productArity: Int = 2
  def productElement(n: Int): Any = if n == 0 then a else b
