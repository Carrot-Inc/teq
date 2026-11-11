//> using options -Werror
trait InfixTypes:
  infix type Pair[A, B] = (A, B)
object InfixSource extends InfixTypes
object InfixMiddle:
  export InfixSource.{Pair as Link}
export InfixMiddle.Link
@main def runExportedInfixTypes(): Unit =
  val pair: Int Link String = (7, "ok")
  println(pair)
