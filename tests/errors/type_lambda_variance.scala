// The parameters of a type lambda take no variance annotation, as under scalac.
object Foo:
  type T[+A] = [+B] =>> (A, B)
// expect: no `+/-` variance annotation allowed here
