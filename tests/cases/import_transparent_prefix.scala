// An import through a transparent inline method is through its expansion (`Typer.typedImportQualifier`,
// `Inlines.inlineCall` in `Typer.adapt`): an object's path, a given's path.
object O { val x = 1 }
class C { val y = 2 }
transparent inline def pure: Any = O
transparent inline def viaGiven(using c: C): c.type = c
given C = new C
object Use {
  import pure.x
  import viaGiven.y
  val a = x + y
}
@main def run(): Unit = println(Use.a)
