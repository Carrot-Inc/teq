// A binder's `@nowarn` of a local constant, a parameter's singleton and an import's rename silences the bound's
// deprecation under `-Werror` (the review of `50e3a1b38`'s NowarnLocal, NowarnParam and NowarnAlias).
import scala.annotation.nowarn
import scala.annotation.{nowarn as quiet}
@deprecated("old", "1") class Old

def local = {
  val filter: "cat=deprecation" = "cat=deprecation"
  type L = [@nowarn(filter) A <: Old] =>> List[A]
  1
}
def param(filter: "cat=deprecation") = {
  type L = [@nowarn(filter) A <: Old] =>> List[A]
  1
}
type Renamed = [@quiet("cat=deprecation") A <: Old] =>> List[A]

@main def run(): Unit = println(local + param("cat=deprecation"))
