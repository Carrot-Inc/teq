// A Scala 3 library of universal traits whose first parent is an alias of `Any` (tests/support/jars.sh's
// universal-lib, compiled by scalac, whose pickles name the alias: `trait U extends Types.Root`), and a trait extending
// one without `Any`, an `AnyRef` (tests/classpath/universal_alias_lib).
package unilib

object Types { type Root = Any }
type TopRoot = Any
trait Selected extends Types.Root
trait Top extends TopRoot
trait Below extends Selected
