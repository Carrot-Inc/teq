// Lambdas with no expected type whose body gives dotty no callee to type the parameter from
// (`Typer.typedFunctionValue`'s `calleeType`): not every parameter among the arguments, and a function part
// naming the parameter. scalac 3.8.4: E081, Missing parameter type, for each.
// expect: 11:15: error: missing parameter type
// expect: 11:18: error: missing parameter type
// expect: 12:13: error: missing parameter type
final case class Tag(name: String):
  def wrap(t: Tag): Tag = Tag(name + t.name)
def pair(name: String, n: Int): Tag = Tag(name + n)

val notAll = (s, n) => pair(s, 1)
val named = x => x.wrap(x)
