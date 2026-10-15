// A macro that counts its expansions, the argument of the deeper selection's member: typed once, as dotty's
// `FunProto.typedArgs` caches it for the application (`typedArg`), the macro expanded once.
import scala.quoted.*
object M:
  var count: Int = 0
  inline def arg: Int = ${ impl }
  def impl(using Quotes): Expr[Int] =
    count += 1
    Expr(count)
