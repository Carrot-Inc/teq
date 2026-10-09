package gaa

// Old-style abstract givens (`given x: T`) across a module boundary: abstract defs flagged `Given`
// in the trait's pickle and abstract methods in its interface, as scalac writes them, which a
// downstream class implements by a given, a def or a val, and which the trait's own body summons.
trait Decls:
  given x: Int
  given label(using n: Int): String
  def twice: Int = summon[Int] * 2
