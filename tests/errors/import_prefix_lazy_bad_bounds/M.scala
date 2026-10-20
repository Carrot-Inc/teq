// A final lazy val's path is realizable where its type is (`CheckRealizable.realizability`: a late-initialized
// member takes its info's), and `T`'s member `A` has bounds that conflict (`boundsRealizability`).
// expect: U.scala:1:8: error: (O.x : T) is not a legal path since its underlying type (O.x : T) has a member A with possibly conflicting bounds Any <: ... <: Nothing
trait T:
  type A >: Any <: Nothing
object O:
  final lazy val x: T = ???
