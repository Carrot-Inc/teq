// `B.use` calls `Later.get()`, leaving its parameter to its default, whose getter the call completes
// (`Applications.tryDefault`'s `defaultArg.tpe.widen`), typing the default and reporting its error, before
// `B.dependent` reads `A.inferred`, which a transparent macro of the run suspended: the getter's report stands, as
// dotty's completer typed it when read and nothing retracts it. `Later` comes after `B`'s cut.
// expect: B.scala:5:40: error: type mismatch: found 123, required String
// expect: Z.scala:2:24: error: type mismatch: found 456, required Boolean
// expect: 2 errors found
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
