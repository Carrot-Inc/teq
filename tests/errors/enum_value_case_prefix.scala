// An enum nested in a class is a type per enclosing instance, its value cases typed through their
// companion's path as dotty's `TermRef(b.E, A)` widens to `b.E` (an enum nested in a class is no static
// class): `b.E.A` is no `a.E`. Master took an enum nested in a class as one type.
class O:
  enum E:
    case A
val a = new O
val b = new O
val x: a.E = b.E.A
// expect: 9:14: error: type mismatch: found (b.E.A : E), required a.E
