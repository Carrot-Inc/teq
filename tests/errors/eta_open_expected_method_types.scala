// expect: 9:26: error: type mismatch: found Long => Long, required Int => Any
// expect: 1 error found
// An expected function type that is not fully defined (`map`'s result open) leaves the eta-expansion
// the method's own parameter types (dotty's adapt1 gives etaExpand no arity), so a `Long` parameter
// does not take the list's `Int` elements.
def longer(x: Long): Long = x * 2

@main def run(): Unit =
  println(List(1, 2).map(longer))
