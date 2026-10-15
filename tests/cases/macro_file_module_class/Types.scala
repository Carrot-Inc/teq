package ident
opaque type Token = Int
object Token:
  def apply(x: Int): Token = x
extension (x: Token) def twice: Int = x * 2
object Top
