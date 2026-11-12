// A lambda that a block's statement starts, after another statement or as a case's body, takes the rest of
// the block as its body (dotty's `Parsers.closureRest` in `Location.InBlock`): the body is a block and no
// application, so nothing but `Any` types the parameter the expected type leaves open
// (`Typer.typedFunctionValue`'s `calleeType`). scalac 3.8.4: `Found: (x : Any)  Required: String` for each.
// expect: 13:12: error: type mismatch: found Any, required String
// expect: 15:48: error: type mismatch: found Any, required String
final case class Tag(name: String)
def one(name: String): Tag = Tag(name)
def on[B, R](f: B => R): B => R = f

val afterStatement = on {
  println("side")
  x => one(x)
}
val caseBody = on(1 match { case _ => x => one(x) })
