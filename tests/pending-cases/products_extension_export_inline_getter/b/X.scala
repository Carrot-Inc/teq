// A downstream of a/ as tests/modules builds it, whole and over teq's products. Through a companion extension method's
// export the forwarder's getters are numbered for its member (`foreachDefaultGetterOf`, Namer.scala:1249-1267) while
// the zip moves the parameters' `HasDefault` (Namer.scala:1392-1398): `h2("x")(a = 5)`'s omitted `b` takes the
// member's `h2$default$2`, `a`'s default 3 (235), and `r("x")(4)(c = 9)`'s `b` takes `c`'s, `a + 2` (2469).
// Each run, JVM, JavaScript and interpreter, whole and over the products, prints the .expected, scalac 3.8.4's.
// Over the products it threw NoSuchMethodError on `Ops.h2$default$2` until the change that expands a product's
// inline getter as an inline call. What fails is the whole build's classes against the split build's
// (tests/modules' classes, subset and bundle checks):
// - `h2`: over the products the forwarder's getter is an inline call of its pickled body, the member's getter on the
//   forwarder's path (`Namer.addForwarder`'s `registerInlineInfo`, Namer.scala:1374-1379), whose receiver the
//   expansion binds, as scalac's class does (`aload_1; astore 4` before `ops("x")`). The whole build reads that
//   getter on the call's receiver (`forwarder_getter_body`, `own_getter_expansion` in src/typer/inline.rs) and
//   binds no receiver.
// - `r`: over the products `b`'s default is bound to a local (`bipush 6; istore_3`), where scalac's class and the
//   whole build's put 6 in place. Its cause is not located yet.
// It moves to tests/modules with the change that makes the two builds one. a/ writes `s.concat(x.toString)`: with
// `s + x` (or `s + x.toString`) teq's TASTy writer withholds `ops`'s body ("a concatenation whose left operand is no
// string or number"), overloadexpand's queued item; and the class takes `@nowarn` for scalac's E208 on the export.
import lib.*
@main def run(): Unit =
  println(WCG[Int](7).h2("x")(a = 5))
  println(WCG[Int](7).r("x")(4)(c = 9))
