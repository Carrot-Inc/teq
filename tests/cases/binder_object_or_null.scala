// A binder over `Obj.type | Null` keeps the `Null`: dotty simplifies the binder's type (`Typer.typedBind`), and
// `Null` is not below an object's type, so the union stays and a `null` scrutinee fails the type test below.
// scalac 3.8.4 prints `other`.
trait Named
case object Obj extends Named
def a(v: Obj.type | Null, flag: Boolean): String = v match
  case y => y match
    case _: Named if flag => "named"
    case _ => "other"
@main def run(): Unit = println(a(null, true))
