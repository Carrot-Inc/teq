// The marker marks under its own name: renamed by its import it is the compile-time-only error,
// with scalac's note (dotty's `Erasure`, 536-543).
// expect: 6:26: error: `deferred` can only be used as the right hand side of a given definition in a trait.
// expect: Note that `deferred` can only be used under its own name when implementing a given in a trait; `later` is not accepted.
import scala.compiletime.{deferred as later}
trait T { given x: Int = later }
