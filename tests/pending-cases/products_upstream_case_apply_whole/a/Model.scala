package tnba

// The application's shape: a case class whose written companion runs a body (a lazy given, so it is
// no `NoInits` class from source, and is one read back from a pickle), an opaque type's companion,
// and a case class over both.
final case class SpaceId(value: Int)
object SpaceId:
  given Ordering[SpaceId] = Ordering.by(_.value)

object Keys:
  opaque type Key = String
  object Key:
    def apply(s: String): Key = s
    given Ordering[Key] = Ordering.String
    extension (k: Key) def value: String = k

final case class Named(key: Keys.Key, id: SpaceId)
