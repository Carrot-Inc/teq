// Two alternatives of an overloaded own `==` that a second module may not access, each called from an inline
// method of the class: the second module's expansions call them through the products.
package lib

class Over(val n: Int):
  def ==(o: Int): Boolean = { print("int "); true }
  private def ==(o: Over): Boolean = { print("priv "); true }
  private[lib] def ==(o: Char): Boolean = { print("pkg "); true }
  inline def self(o: Over): Boolean = this == o
  inline def chr: Boolean = this == 'c'
