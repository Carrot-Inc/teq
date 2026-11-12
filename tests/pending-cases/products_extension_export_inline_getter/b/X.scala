// A downstream of a/ as tests/modules builds it, whole and over the products. Through a companion extension method's
// export the forwarder's getters are numbered for its member (`foreachDefaultGetterOf`, Namer.scala:1249-1267) while
// the zip moves the parameters' `HasDefault` (Namer.scala:1392-1398): `h2("x")(a = 5)`'s omitted `b` takes the
// member's `h2$default$2`, `a`'s default 3 (235), and `r("x")(4)(c = 9)`'s `b` takes `c`'s, `a + 2` (2469).
// Each run, JVM, JavaScript and interpreter, whole and over the products, is to print the .expected, scalac 3.8.4's.
// The whole build's classes are to be
// the split build's (tests/modules' classes,
// subset and bundle checks):
// - `h2`: the expansion of the forwarder's getter is to bind,
//   in both builds, the receiver of the member's getter
//   on the forwarder's path (`Namer.addForwarder`'s
//   `registerInlineInfo`, Namer.scala:1374-1379), as scalac's
//   class does (`aload_1; astore 4` before `ops("x")`).
// - `r`: both builds are to put `b`'s default 6 in place,
//   as scalac's class does.
// It moves to tests/modules with the change that makes
// the two builds one. The class takes `@nowarn`
// for scalac's E208 on the export.
import lib.*
@main def run(): Unit =
  println(WCG[Int](7).h2("x")(a = 5))
  println(WCG[Int](7).r("x")(4)(c = 9))
