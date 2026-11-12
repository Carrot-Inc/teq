// A default getter's name is `DefaultGetterName(h, 1)`, a derived name
// (NameKinds.scala:358), which no identifier the source writes is: `h$default$2`
// reads as the simple name of that spelling, and two names are equal only
// as the same name (Names.scala:157), so a selection of that spelling
// finds no member and scalac 3.8.4 reports E008, as it does for a library's
// getter (`Ops("x7").h2$default$1`). Found by the workers exports helper.
class O { def h(a: Int = 1, b: Int = 2): Int = a + b }
@main def run(): Unit =
  println(O().h$default$2)
