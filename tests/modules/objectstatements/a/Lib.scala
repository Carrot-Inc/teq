package osa

// Objects by where their initialisation's effect lives: one set is named by a bare name downstream, the other
// through the package.
object OwnN { println("  own statement (by name)") }
object OwnP { println("  own statement (through the package)") }
class LoudN { println("  inherited body (by name)") }
class LoudP { println("  inherited body (through the package)") }
object InheritedN extends LoudN
object InheritedP extends LoudP
trait BodyN { println("  trait body (by name)") }
trait BodyP { println("  trait body (through the package)") }
object ViaTraitN extends BodyN
object ViaTraitP extends BodyP
class DfltP(x: Int = { println("  omitted default (through the package)"); 1 })
object DefaultP extends DfltP()
object QuietP
