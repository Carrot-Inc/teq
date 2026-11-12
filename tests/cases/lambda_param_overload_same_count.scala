// The callee has two alternatives taking one argument: nothing but the count decides before the argument is
// typed, so the lambda's open parameter is `Any` and the alternative taking `Any` is the one called.
final case class Tag(name: String)
def pick(name: String): Tag = Tag("string:" + name)
def pick(any: Any): Tag = Tag("any:" + any)
def on[B, R](f: B => R): B => R = f

@main def run(): Unit =
  val placeholder = on(pick(_))
  val named = on(x => pick(x))
  println(placeholder("a").name + " " + named("b").name)
