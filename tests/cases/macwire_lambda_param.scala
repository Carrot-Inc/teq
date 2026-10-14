//> using dep com.softwaremill.macwire::macros:2.6.7
// jars: macwire-macros macwire-util
// `wire` inside a lambda takes the lambda's parameter: `Symbol.tree` of the `$anonfun` around the
// call is a `DefDef` with its parameters (dotty's `FromSymbol.definitionFromSym`), and the call in
// the local `mk` of an untyped `val` expands once the val's type is set (dotty's `Inlining`, after
// typer), so macwire's walk of the class's members reads `api`'s type.
import com.softwaremill.macwire.*

class Coll(val name: String)
class Api(coll: Coll):
  def show = s"api(${coll.name})"
class Index(coll: Coll, api: Api):
  def show = s"index(${coll.name}, ${api.show})"

class Env:
  val api =
    def mk = (coll: Coll) => wire[Api]
    mk(Coll("users"))
  val index = List(Coll("games")).map(c => wire[Index]).head
  val viaFor = for coll <- List(Coll("posts")) yield wire[Api]

@main def run(): Unit =
  val env = new Env
  println(env.api.show)
  println(env.index.show)
  println(env.viaFor.map(_.show))
