// A link-time property is the linker's to answer, no constant of the language: an `inline if` over it is refused,
// over `productionMode` and over `developmentMode` (`!productionMode`) alike, as scalac 3.8.4 --js has it (L:C is
// where its caret begins, 1-based).
// expect: 11:11: error: Cannot reduce `inline if` because its condition is not a constant value:
// expect: 12:11: error: Cannot reduce `inline if` because its condition is not a constant value:
// expect: 2 errors found
import scala.scalajs.LinkingInfo
inline def mode: String = inline if LinkingInfo.productionMode then "production" else "development"
inline def dev: String = inline if LinkingInfo.developmentMode then "development" else "production"
@main def run(): Unit =
  println(mode)
  println(dev)
