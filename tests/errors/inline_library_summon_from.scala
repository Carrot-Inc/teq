// jars: scala-library
// teq: --target jvm --std=scala-library
// expect: 7:17: error: cannot reduce summonFrom with
// expect: 1 error found
// An expansion's own error in a library body, `Predef.valueOf`'s summonFrom that no given reduces, is the
// program's error at the call (`InlineTyper.typedMatchFinish`), without the naming of a body teq typed.
def v: String = valueOf[String]
@main def run(): Unit = println(v)
// absent: (in the body of
