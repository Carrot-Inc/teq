// A companion's inline method that calls a private method of the class: a second module expands it over this
// module's products and over scalac's build of it.
package lib

class Priv(val n: Int):
  private def eqv(o: Priv): Boolean = true

object Priv:
  inline def both(a: Priv, b: Priv): Boolean = a.eqv(b)
