// A custom interpolator's context is the application of StringContext's name (Desugar's InterpolatedString arm:
// Apply(scalaDot(nme.StringContext), strs)), which a macro's quote pattern over that application matches, as it matches
// the application written out.
@main def run(): Unit =
  println(words"four five")
  println(StringContext("a b c").words())
