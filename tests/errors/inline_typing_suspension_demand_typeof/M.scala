// `type Use = Later.bad.type` types the path as an expression (`Typer.typedSingletonTypeTree`), whose
// stability check reads `Later.bad`'s info and completes it, typing its body: the report stands, as the
// traversal reaches `Use` before `B.dependent` suspends `B`.
// expect: B.scala:5:33: error: type mismatch: found 123, required String
// expect: Z.scala:2:24: error: type mismatch: found 456, required Boolean
// expect: 2 errors found
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
