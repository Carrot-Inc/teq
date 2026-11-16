package cta

// A module scalac compiles against the upstream, run over the upstream's class files: the constructors
// their qualifier lets this package call are there to link against.
object Client:
  def attempt(name: String)(f: => Any): Unit =
    try println(name + ": " + f)
    catch case e: LinkageError => println(name + ": " + e.getClass.getSimpleName)

  def main(args: Array[String]): Unit =
    attempt("primary")(new P3(1).x)
    attempt("secondary")(new S().x)
    attempt("protected")(new S("four").x)
    attempt("hidden")(new Hidden("argument").x)
    attempt("case")(K(5).copy(x = 6))
