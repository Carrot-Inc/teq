/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "scala.collection.IterableOps.init"
]
lint.error.includes = ".*"
*/
package fix

object UnusedAnchorError:
  val xs = List(1, 2, 3)
  val reported = xs.init // assert: ForbiddenSymbol
  val anchored = xs.init // scalafix:ok ForbiddenSymbol
  val anchoredBare = xs.init // scalafix:ok
  val unused = xs.head /* scalafix:ok ForbiddenSymbol */ // assert: UnusedScalafixSuppression
  val unusedBare = xs.size /* scalafix:ok */ // assert: UnusedScalafixSuppression
