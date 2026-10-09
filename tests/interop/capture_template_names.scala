// A `@js` template's own bindings around a placeholder, a parameter named by a contextual word
// (`get`) or destructured (`[x]`), a `var` (through its whole function), the code of a template
// literal's interpolation and the name of a function expression, which binds in its own body
// alone, are read as the template writes them: a local named alike is named apart from what the
// template reads.

@js("((get) => $0)(1)") def throughGet(x: Int): Int = x
@js("(([x]) => $0)([1])") def throughPattern(x: Int): Int = x
@js("`${Math.abs($0)}`") def absolute(x: Int): String = x.abs.toString
@js("(function Math() {}, Math.imul($0, $1))") def mul(a: Int, b: Int): Int = a * b
@js("(() => { if (true) { var probeCapture = 0; } return $0; })()") def throughVar(n: Int): Int = n

@main def main(): Unit =
  val get = 10
  println(throughGet(get))
  val x = 20
  println(throughPattern(x))
  val Math = 9
  println(absolute(-3))
  println(mul(2, 3))
  println(Math)
  val probeCapture = 30
  println(throughVar(probeCapture))
