// expect: 15:31: error: type mismatch: found C, required AnyRef{def f(): Unit}
// expect: 16:29: error: type mismatch: found D, required B{def g(): String}
// A member whose `@targetName` renames it is not the member a refinement of its name asks for
// (`TypeComparer.hasMatchingMember`'s `qualifies`: `m.symbol.hasTargetName(m.symbol.name)`),
// whether a parent declares it or not.
import scala.annotation.targetName
class C:
  @targetName("other") def f(): Unit = ()
trait B:
  @targetName("renamed") def g(): Any
class D extends B:
  @targetName("renamed") def g(): String = "d"
def use(c: AnyRef { def f(): Unit }): Unit = ()
def test(b: B { def g(): String }): String = b.g()
@main def run(): Unit = { use(new C) }
def run2(): String = { test(new D) }
