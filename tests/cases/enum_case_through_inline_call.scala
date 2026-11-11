// An enum case passed to an inline method, `id[Col](Col.Red)` and the others, its parameter bound to the argument's
// singleton type (Inliner.scala:325-327): the inliner reads it as `singleton` of that type, an Ident whose executable
// view (`DesugaredSelect`, tpd.desugarIdent) selected the case on the companion, an accessor the enum does not have,
// where scalac reads the static field (Namer.scala:274's JavaStatic, BCodeBodyBuilder.scala:439-441). The interpreter
// printed `null`, Scala.js `undefined` and `()`, and the JVM build threw NoSuchMethodError on `Col$.Red()`; master
// efddb38de3 printed the .expected, a regression of the type-representation push. Found by the interim reviewer of
// the workers exports helper's push 10.
enum Col:
  case Red
  case Green(i: Int)

transparent inline def id[T](y: T): T = y
transparent inline def tr[T](y: T): String = y.toString
inline def il[T](y: T): String = y.toString
inline def mono(y: Col): Col = y
inline def byInline(inline y: Col): Col = y

@main def run(): Unit =
  println(id[Col](Col.Red))
  println(tr[Col](Col.Red))
  println(il[Col](Col.Red))
  println(mono(Col.Red))
  println(byInline(Col.Red))
  println(mono({ Col.Red }))
  println(id(Col.Green(1)))
  val c = Col.Red
  println(id(c))
