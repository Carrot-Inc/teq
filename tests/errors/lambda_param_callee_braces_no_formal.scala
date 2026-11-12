// A lambda alone in braces with no expected type: the body of a closure a block's statement starts is a
// block (dotty's `Parsers.closureRest` in `Location.InBlock`), no application the parameter's type is read
// from (`Typer.typedFunctionValue`'s `calleeType`). scalac 3.8.4: E081, Missing parameter type.
// expect: 8:11: error: missing parameter type
final case class Tag(name: String)
def one(name: String): Tag = Tag(name)

val g = { x => one(x) }
