// A pattern definition of no variable is makePatDef's general arm (Desugar.scala 1520-1670), checked against 3.8.4
// and pinned to M10 with the bound whole (makePatDef's shape):
// - `val _ = e`: TreeInfo.scala:518's IdPattern refuses a plain wildcard, so it binds no variable and lowers to
//   `e match { case _ => () }` (Desugar.scala:1613): no val, no field;
// - `val _: T = e` is an IdPattern through the ascription and takes `WildcardParamName.fresh()` (1520): a val `_$N`;
// - `lazy val _ = e` takes 1613's Lazy exception: the holder `lazy val $N$ = e match { case _ => () }`.
// teq keeps a val `unnamed$N` for each (src/parser/defs.rs:400). Numbers are masked by the macros.
def side(): Int = 7
class C:
  val _ = side()
  val _: Int = side()
  lazy val _ = side()
@main def run(): Unit =
  println(Shape.names { val _ = side(); 1 })
  println(Shape.names { val _: Int = side(); 1 })
  println(Shape.names { lazy val _ = side(); 1 })
  println(Shape.fields[C])
