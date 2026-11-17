// The own `==` and `!=` of a class named through a path inside a library module's own bodies, a method's and an
// inline method's: a second module calls them over this one's products and over scalac's build of it, where the
// inline bodies are typed again at the call.
package lib

class Ops(val seed: Int):
  object nums:
    class Money(val n: Int) { def ==(o: Money): Boolean = n == o.n; def !=(o: Money): Boolean = n != o.n }
  def same: Boolean = new nums.Money(1) == new nums.Money(1)
  def differ: Boolean = new nums.Money(1) != new nums.Money(1)
  inline def inlined: Boolean = new nums.Money(1) == new nums.Money(1)
  inline def expanded: Boolean = new nums.Money(1) != new nums.Money(2)
