// A lambda with no expected type whose parameter stands twice among the callee's arguments: dotty's
// `paramIndices` gives an index only to a parameter that is an argument exactly once
// (`Typer.typedFunctionValue`), so nothing types it. scalac 3.8.4: E081, Missing parameter type.
// expect: 7:9: error: missing parameter type
def two(a: String, b: String): String = a + b

val f = x => two(x, x)
