package eoa

// Value cases before a class case: the ordinals follow the source, which the pickle lists
// otherwise (the class case's definition first).
enum Kind:
  case Small, Large
  case Sized(n: Int)
  case Huge
