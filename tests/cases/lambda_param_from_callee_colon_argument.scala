// An argument on an indented line after the callee's colon is the expression itself (dotty's
// `Parsers.blockExpr` simplifies an indented region), an identifier that names the lambda's parameter; one in
// parentheses is a node of its own and no occurrence, so the parameter beside it stands once.
final case class Tag(name: String)
def one(name: String): Tag = Tag(name)
def two(a: String, b: String): Tag = Tag(a + b)
def on[B, R](f: B => R): B => R = f
@main def run(): Unit =
  val none = x => one:
    x
  val open = on(x => one:
    x)
  val parens = x => two(x, (x))
  println(none("a").name + " " + open("b").name + " " + parens("c").name)
