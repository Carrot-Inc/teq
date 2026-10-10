package scala.reflect

// Every enum class and enum case is an `Enum`, as a case class is a `Product`: the subtype rule
// is the compiler's (`Typer::is_enum_value`) and the members are what an enum value answers, so
// the trait is what a macro's `E & reflect.Enum` reaches for.
trait Enum extends Product:
  // Also `java.lang.Enum.ordinal()` of an enum over it, which takes the parentheses. An abstract
  // member (dotty's `scala.reflect.Enum`): a call dispatches to the receiver's implementation,
  // an enum case's being the synthesized one reading its `$ordinal` (`$enumOrdinal`).
  @javaDefined
  @js("$enumOrdinal($0)")
  def ordinal: Int
