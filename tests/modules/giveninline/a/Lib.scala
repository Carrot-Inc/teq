package gina

// Inline givens of an object read over the products: a plain one (scalac pickles its right-hand side
// as the inline body and keeps an `ELIDED` val) and one with an effect, expanded at each summon.
object O:
  inline given n: Int = 42
object E:
  var count = 0
  inline given c: Long = { count += 1; count.toLong }
