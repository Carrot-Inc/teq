package efa

// An enum's cases, value and class, read over the products by a downstream module.
enum Conn:
  case And, Or
  case Not(c: Conn)
