// A top-level val written before `object B` is a member of the file's package object, which dotty's desugaring
// places after `B` (`Desugar.packageDef`): `B` suspends at `A.inferred` before the val is typed, and its error is
// never reported in that run, which `Z`'s error ends: scalac reports `Z`'s alone.
// expect: Z.scala:2:24: error: type mismatch: found 456, required Boolean
// expect: 1 error found
// absent: B.scala:1:21
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
