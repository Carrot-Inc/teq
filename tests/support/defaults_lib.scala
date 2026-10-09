// The jar of tests/classpath/jvm/jar_ctor_defaults.scala: constructors with defaults, whose getters
// scalac writes on the companion, taking the clauses before the default's only, with static
// forwarders on a top-level class (circe-yaml's `Printer`) and none on a nested one.
package defaultslib

final case class Printer(preserveOrder: Boolean = false, dropNullKeys: Boolean = false, indent: Int = 2, name: String = "p")

class Box(val a: Int, val b: Int = 7)(val c: Int = a + b)

object Holder {
  final case class Inner(x: Int = 1, y: String = "y")
}
