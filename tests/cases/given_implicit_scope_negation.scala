// The implicit scope's candidates are an eligible list of their own (`searchImplicit`'s
// `implicitScope(wildProto).eligible`), which no name of the context hides: an imported `sh` that
// matches and fails leaves the companion's `sh`; and a `NotGiven` negates a companion's candidate
// as it negates the context's (`negateIfNot`), here one whose using clause fails.
import scala.util.NotGiven
import scala.compiletime.summonFrom
trait Show[A] { def show: String }
trait Missing
class Foo
object Foo:
  given sh: Show[Foo] with { def show = "companion" }
  given nf(using Missing): NotGiven[Foo] = ???
object L:
  given sh(using Missing): Show[Foo] = ???
inline def ask: String = summonFrom {
  case _: NotGiven[Foo] => "absent"
  case _ => "present"
}
@main def run(): Unit =
  import L.given
  println(summon[Show[Foo]].show)
  given Foo = new Foo
  println(ask)
