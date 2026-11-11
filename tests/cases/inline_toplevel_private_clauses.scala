// An inline method of a package calls a private method of the package, which it reaches from where it is inlined
// through an accessor (`PrepareInlineable`): the accessor is a definition of the package, and a call of two
// parameter clauses stays two applications.
inline def twice(n: Int): Int = sum(n)(n)
private def sum(n: Int)(m: Int): Int = n + m

inline def bump(n: Int): Int = next(n)
private def next(n: Int): Int = n + 1

inline def scaled(n: Int)(using factor: Int): Int = times(n)
private def times(n: Int)(using factor: Int): Int = n * factor

@main def run(): Unit =
  println(twice(21))
  println(bump(41))
  given Int = 6
  println(scaled(7))
