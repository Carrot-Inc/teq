// As meet_overloads_arity_first, the two parts generic: an overload takes the part's own type
// parameter, seen through each intersection's instance.
trait Reads[T]:
  def at(key: String): String
  def at(slot: T): Int
