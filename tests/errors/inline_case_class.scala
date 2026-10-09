// A case class cannot be defined in an inline method that is a member, as under scalac 3.8.4: its
// E162 at the definition, once, called twice or not.
inline def species() =
  case class FooT()
  FooT()
val foo = species()
val bar = species()
// expect: 4:14: error: Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead.
// expect: 1 error found
