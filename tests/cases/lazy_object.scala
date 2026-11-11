// `lazy` on an object, which scalac accepts: the module class keeps only the flags a module class retains
// (`Desugar.moduleDef`, `mods.toTypeFlags & RetainedModuleClassFlags | ModuleClassCreationFlags`), so that the term's
// `Lazy` never reads as the class's `Trait`, and the products' pickles are written (the M3 adversary's probes n05,
// n13 and n14, where writing them panicked on the kinds).
package lazy_object

lazy object Top { val x = 1 }
lazy case object CO
implicit lazy object IO { def v = 2 }
private lazy object PO { def w = 3 }

class Holder:
  lazy object Inner { val y = 4 }

object Outer:
  lazy object Nested { val z = 5 }

trait T:
  lazy object InTrait { val t = 6 }

@main def main(): Unit =
  println(Top.x)
  println(CO)
  println(IO.v)
  println(PO.w)
  println(new Holder().Inner.y)
  println(Outer.Nested.z)
  println(new T {}.InTrait.t)
  lazy object Loc { val l = 7 }
  println(Loc.l)
