// `Symbol`: interned by name, hashed by its name, its name and its text; what a pretty-printer's
// match on it reads.
@main def run =
  val a = Symbol("alpha")
  println(a.name)
  println(a)
  println(a eq Symbol("alpha"))
  println(a == Symbol("beta"))
  val shown = (a: Any) match
    case s: Symbol => "'" + s.name
    case other => other.toString
  println(shown)
  println(Symbol("hello").hashCode == "hello".hashCode)
  println(Set(Symbol("a"), Symbol("a"), Symbol("b")).size)
