package objs

object OwnN { println("  own statement (by name)") }
class LoudN { println("  inherited body (by name)") }
object InheritedN extends LoudN
class ArgN(x: Int)
object ParentArgumentN extends ArgN({ println("  parent's argument (by name)"); 1 })
class SecN(x: Int) { def this() = this({ println("  secondary's argument (by name)"); 1 }) }
object SecondaryN extends SecN()
class DfltN(x: Int = { println("  omitted default (by name)"); 1 })
object DefaultN extends DfltN()
object QuietN
object LazyN { lazy val x = { println("  lazy val (by name)"); 1 } }
object HolderN { object In { println("  nested object (by name)") } }
trait BodyN { println("  trait body (by name)") }
object ViaTraitN extends BodyN
class MixedN extends BodyN
object ViaClassN extends MixedN
case object CaseArgumentN extends ArgN({ println("  case object's argument (by name)"); 2 })

object OwnP { println("  own statement (through the package)") }
class LoudP { println("  inherited body (through the package)") }
object InheritedP extends LoudP
class ArgP(x: Int)
object ParentArgumentP extends ArgP({ println("  parent's argument (through the package)"); 1 })
class SecP(x: Int) { def this() = this({ println("  secondary's argument (through the package)"); 1 }) }
object SecondaryP extends SecP()
class DfltP(x: Int = { println("  omitted default (through the package)"); 1 })
object DefaultP extends DfltP()
object QuietP
object LazyP { lazy val x = { println("  lazy val (through the package)"); 1 } }
object HolderP { object In { println("  nested object (through the package)") } }
trait BodyP { println("  trait body (through the package)") }
object ViaTraitP extends BodyP
class MixedP extends BodyP
object ViaClassP extends MixedP
case object CaseArgumentP extends ArgP({ println("  case object's argument (through the package)"); 2 })
