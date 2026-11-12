/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "scala.Option.get",
  "fix.SupplementaryName.`𝔘box`.value"
]
*/
package fix

// A name with a character outside the basic plane is no Java identifier (a surrogate pair is two units, neither a
// letter), so a symbol has it between backquotes: fix/SupplementaryName.`𝔘get`. and
// fix/SupplementaryName.`𝔘box`.value().
object SupplementaryName:
  val 𝔘get = 1
  val sum = 𝔘get + 1
  object 𝔘box:
    def value: Int = 1 // assert: ForbiddenSymbol
  val box = 𝔘box
  val read = box.value // assert: ForbiddenSymbol
  val option = Option(1).get // assert: ForbiddenSymbol
