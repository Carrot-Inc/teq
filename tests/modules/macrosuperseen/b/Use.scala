// A downstream of a/ that expands a/'s macros, whose implementations the interpreter is to run from a/'s products,
// built by scalac too (over-scalac). scalac signs a super accessor with the accessed member's type as the accessor's
// class sees it (`superInfo`, SuperAccessors.scala:77-81): `(java.lang.String, scala.Int)Int` for `Base[String]`'s
// `f(using a: A)(n: Int)`, whose own signature is `(java.lang.Object, scala.Int)Int`, and
// `(1, java.lang.Object, scala.Int)Int`, a PolyType's, for the polymorphic `f[A](using a: A)(n: Int)`. The two lines
// below call each through a macro of a/: the one whose parameter's type is substituted, then the polymorphic one.
package msb

@main def run(): Unit =
  println(mss.Macros.Mac.seen)
  println(mss.Macros.Mac.poly)
