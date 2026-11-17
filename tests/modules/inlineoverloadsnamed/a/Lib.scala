// Two overloads of one name that a second module may not access, each called from an inline method of the class:
// the second module's expansions call them through the products.
package lib

class Over(val n: Int):
  def eqv(o: Int): Boolean = { print("int "); true }
  private def eqv(o: Over): Boolean = { print("priv "); true }
  private[lib] def eqv(o: Char): Boolean = { print("pkg "); true }
  inline def self(o: Over): Boolean = this.eqv(o)
  inline def chr: Boolean = this.eqv('c')
