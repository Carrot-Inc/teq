// A method may be named `<error>` in backquotes: the name the parser's recovery gives a missing one is that name too,
// and only a missing one leaves the definition incomplete; this one's result is inferred.
class C:
  def `<error>` = 1
@main def run(): Unit = println(C().`<error>` + 1)
