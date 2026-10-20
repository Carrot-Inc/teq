// An import's qualifier that is no stable path is typed as an expression (`Typer.typedImportQualifier`,
// `typedExpr(expr, AnySelectionProto)`) and checked at the statement (`Checking.checkLegalImportOrExportPath`):
// a parameterless method's reference is its `TermRef`, stable where its result is, impure (`refPurity`); a
// method with an empty parameter list must be called with `()`; a transparent inline method's qualifier is its
// expansion (`Typer.adapt`, `Inlines.inlineCall`), an object's or a given's path legal, a `new` not.
// expect: 23:10: error: (f : => Int) is not a valid import prefix, since it is not an immutable path
// expect: 23:10: error: import prefix is not a pure expression
// expect: 24:10: error: import prefix is not a pure expression
// expect: 25:10: error: method h must be called with () argument
// expect: 33:10: error: C is not a valid import prefix, since it is not an immutable path
// expect: 33:10: error: import prefix is not a pure expression
// expect: 6 errors found
object O { val x = 1 }
class C { val y = 2 }
def f: Int = 2
def g: O.type = O
def h(): O.type = O
transparent inline def pure: Any = O
transparent inline def impure: Any = new C
transparent inline def viaGiven(using c: C): c.type = c
given C = new C
object Use1 {
  import f.*
  import g.x
  import h.x
}
object Use2 {
  import pure.x
  import viaGiven.y
  val a = x + y
}
object Use3 {
  import impure.y
}
