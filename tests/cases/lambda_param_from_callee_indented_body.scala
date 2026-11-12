// A lambda's body on an indented line of its own is the expression itself, not a block (dotty's
// `Parsers.blockExpr` simplifies an indented region), so the callee it applies types the parameter the
// expected type leaves open; the same after a colon argument's arrow.
final case class Tag(name: String)
def one(name: String): Tag = Tag(name + "!")
def on[B, R](f: B => R): B => R = f

@main def run(): Unit =
  val indented = on(x =>
    one(x))
  val colon = on: x =>
    one(x)
  println(indented("a").name + " " + colon("b").name)
