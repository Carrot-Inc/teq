// A chain of singleton aliases folds to its end, as scalac widens a reference by its info at every link
// (widenTermRefExpr): ten links, a final lazy end and a non-final literal-typed end, without initialising Q.
object Q {
 println("Q init")
 final val t = 7
 final val alias: t.type = t
 final lazy val l = 8
 inline val viaLazy: l.type = l
 val plain: 9 = 9
 inline val viaPlain: plain.type = plain
 inline val x0 = 10
 inline val x1: x0.type = x0
 inline val x2: x1.type = x1
 inline val x3: x2.type = x2
 inline val x4: x3.type = x3
 inline val x5: x4.type = x4
 inline val x6: x5.type = x5
 inline val x7: x6.type = x6
 inline val x8: x7.type = x7
 inline val x9: x8.type = x8
 inline val x10: x9.type = x9
}
object Edges {
 def main(args: Array[String]): Unit = {
 println(Q.viaLazy)
 println(Q.viaPlain)
 println(Q.x10)
 println(Q.alias)
 }
}
