// A lazy val whose type is an intersection of two instances of a covariant
// class is a realizable import prefix: `CheckRealizable.boundsRealizability`
// walks the base types with the parameter's variance (`baseTypeOf` over
// `baseClasses`, `HasProblemBase` only for an invariant parameter's
// conflicting arguments), so `Box[String] & Box[Any]` has no member
// with conflicting bounds and scalac accepts the import. teq is to
// accept it too. Pinned for the queued part inlining-lila-folds.
trait Box[+A]
object Test { final lazy val box: Box[String] & Box[Any] = ???; import box.* }
@main def main(): Unit = println("ok")
