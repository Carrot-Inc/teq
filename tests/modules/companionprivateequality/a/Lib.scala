// A companion's inline method that calls the class's private own `==`: a second module expands it over this
// module's products and over scalac's build of it; its own `a == b` from outside is the universal equality.
package lib

class Priv(val n: Int):
  private def ==(o: Priv): Boolean = true

object Priv:
  inline def both(a: Priv, b: Priv): Boolean = a == b
