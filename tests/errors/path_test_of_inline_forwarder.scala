// command: build
// expect: error: the inline call paths.Facade.width in paths.kind was not expanded
// A singleton type test, in plain code, over a path through an export's forwarder of an inline val. scalac
// refuses the program, `method width is declared as `inline`, but was not inlined`: its test compares with the
// path's value, a reference it makes after the inline calls are expanded. The build refuses it where the test
// is reached, with the sentence of its check of unexpanded calls and not yet with scalac's.
package paths

object Origin:
  inline val width = 3

object Facade:
  export Origin.width

def kind(x: Any): String = x match
  case _: Facade.width.type => "width"
  case _ => "other"

@main def run(): Unit = println(kind(3))
