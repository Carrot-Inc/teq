// A renamed import of an extension method used by the extension syntax: scalac 3.8.4 matches a
// renamed selector by the name the typer leaves (the method's), so it reports the import; one
// used by its new name as a method, a renamed value and a plain extension import are used.
object A:
  extension (a: Int) def f: Int = a
  extension (s: String) def g: Int = s.length
  val v = 1
object B:
  import A.f as f2
  val x = 2.f2
object C:
  import A.g
  val y = "s".g
object D:
  import A.v as w
  val z = w
object E:
  import A.f as f4
  val q = 2
