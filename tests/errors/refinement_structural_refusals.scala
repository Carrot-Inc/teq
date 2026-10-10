// expect: 11:26: error: Polymorphic refinement method run without matching type in parent type AnyRef is no longer allowed
// expect: 11:55: error: Structural access not allowed on method run because it is polymorphic
// expect: 12:60: error: Structural access not allowed on method run because it has a method type with inter-parameter dependencies
// expect: 13:52: error: method f does not take type parameters
// A new polymorphic member is refused where the refinement is formed (`Typer.typedRefinedTypeTree`)
// and again where it is called structurally, as `Dynamic.handleStructural` refuses a polymorphic
// or a dependent method; a monomorphic one takes no type arguments.
import scala.reflect.Selectable.reflectiveSelectable
trait Ctx { type T }
// the members:
def test(k: AnyRef { def run[T](x: T): T }): String = k.run[String]("ok")
def dep(k: AnyRef { def run(c: Ctx): c.T }, c: Ctx): c.T = k.run(c)
def targs(k: AnyRef { def f(x: Int): Int }): Int = k.f[Int](1)
