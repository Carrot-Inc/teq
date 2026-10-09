// Evidence converts a value (tests/cases/evidence_conversion.scala), not the type it bounds: a
// `List[A]` is no `List[String]` for an `A <:< String`, as scalac refuses it too.
def f[A](xs: List[A])(using ev: A <:< String): List[String] = xs
def g[A](a: A)(using ev: A <:< String): Int = a.length
// expect: 3:63: error: type mismatch: found List[A], required List[String]
// absent: 4:
