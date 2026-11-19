// `B.use` reads `Later.bad`, whose inferred type types its body and reports its error, then `B.dependent` reads
// `A.inferred`, which a transparent macro of the run suspended: `B` suspends there, and what the completion `B.use`
// demanded reported stands, as Namer's completer typed it when read and nothing retracts it, whichever worker
// typed it; `Z`'s error and it end the runs (`Driver.finish`).
// expect: B.scala:5:33: error: Found:    123⏎Required: String
// expect: Z.scala:2:24: error: Found:    456⏎Required: Boolean
// expect: 2 errors found
import scala.quoted.*
object M:
  transparent inline def value: Int = ${ impl }
  def impl(using Quotes): Expr[Int] = Expr(7)
