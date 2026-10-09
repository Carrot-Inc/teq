// The import of `scala.compiletime.deferred` is used by the deferred given it marks, which is
// never typed: the marker is resolved where the given's signature completes, as dotty's `Namer`
// types it ahead. The import beside it is not used.
import scala.compiletime.deferred
import scala.collection.mutable

trait T:
  given x: Int = deferred
