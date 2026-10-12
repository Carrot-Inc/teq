package nea

// Enums nested in a class and in a trait: one companion per enclosing instance, an object nested
// there (eager where the companion is made in a class, lazy for a written one and in a trait),
// its values read through the instance, its members reading the instance.
class O(val n: Int):
  enum E:
    case A, B
    case C(x: Int)
    def get: Int = this match
      case A => n
      case B => n + 1
      case C(x) => n + x

class P:
  enum F:
    case X
  object F:
    def first = X

trait Q:
  enum G:
    case Y
class R extends Q
