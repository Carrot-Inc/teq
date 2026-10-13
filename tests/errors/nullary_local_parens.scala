// expect: 9:11: error: value apply is not a member of String
// expect: 1 error found
// A local `def toString: String` overrides nothing, so `toString()` is no call of it
// (`matchNullaryLoosely` asks the overridden symbols): the parentheses apply its result, as
// scalac's `StringOps.apply` (whose argument is missing there; the lean std's `String` has no
// `apply`).
@main def run(): Unit =
  def toString: String = "local"
  println(toString())
