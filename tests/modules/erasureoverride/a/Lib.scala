// The members a downstream overrides and implements: their declarations erase as scalac's (`TypeErasure`), a union of a
// value class with itself to `Object` and a `*:` chain behind an alias to `Product`, read so from the products' pickles
// as from scalac's (the loader's declared views), so the downstream's members override them.
package oa
class V(val x: Int) extends AnyVal
object Types:
  type C = Int *: String *: EmptyTuple
  type U = V | V
class Base:
  def alias(x: Types.C): Types.C = x
  def union(x: Types.U): Types.U = x
  def plain(x: (Int, String)): (Int, String) = x
trait Tr:
  def t(x: Types.C): Types.C
