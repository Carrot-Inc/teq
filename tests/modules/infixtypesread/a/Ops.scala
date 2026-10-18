package itra

// An `infix` alias and class read from the upstream pickles: the downstream's infix uses of them are
// silent, that of an alias not declared so warned of at the default version (`AlphanumericInfix`).
infix type or[A, B] = Either[A, B]
infix class Both[A, B](val a: A, val b: B)
type plain[A, B] = Either[A, B]
trait Members:
  infix type N[A, B]
  type M[A, B]
