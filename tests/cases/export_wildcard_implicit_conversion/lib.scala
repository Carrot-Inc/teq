package lib

// An implicit class's conversion goes with a wildcard export, an implicit still, through an
// object whose parent exports it and a package's export of the object's members, as lila's
// `lila.core` exports `Core.*` whose `CoreExports` exports `scala.concurrent.duration.DurationInt`.
object ext:
  extension [A](a: A) def some: Option[A] = Some(a)
  implicit class Twice(n: Int):
    def twice: Int = n * 2
  given Ordering[Boolean] = Ordering.Boolean

trait CoreExports:
  type Fu[A] = List[A]
  export ext.{ given, * }
  export scala.concurrent.duration.{ DurationInt, FiniteDuration }

object Core extends CoreExports
