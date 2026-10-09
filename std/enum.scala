package scala.reflect

// Every enum class takes `Enum` among its parents, as dotty's `Desugar.classDef` gives it, and
// so every enum case is one: the members are what an enum value answers.
trait Enum extends Any, Product, java.io.Serializable:
  // Also `java.lang.Enum.ordinal()` of an enum over it, which takes the parentheses. An abstract
  // member (dotty's `scala.reflect.Enum`): a call dispatches to the receiver's implementation,
  // an enum case's being the synthesized one reading its `$ordinal` (`$enumOrdinal`).
  @javaDefined
  @js("$enumOrdinal($0)")
  def ordinal: Int
