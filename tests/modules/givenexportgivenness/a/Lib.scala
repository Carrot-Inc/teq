package gega

// A name of an upstream object with a given alternative and a method alternative, exported
// downstream by givenness (dotty's `Namer.addWildcardForwarders`): read over the products.
object O:
  given x: Int = 1
  def x(s: String): String = s
