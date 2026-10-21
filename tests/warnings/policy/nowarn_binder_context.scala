// A binder's `@nowarn` registered from its annotation typed in the binder's context (`completeAnnotations`'
// `registerNowarn`): a local constant's filter, a parameter's singleton, the class through an import's rename; an
// argument of no constant string warns and filters nothing, none filters everything.
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
type Empty = [@nowarn() A <: Old] =>> List[A]
def nonConstant(s: String) = { type L = [@nowarn(s) A <: Old] =>> List[A]; 1 }

@main def run(): Unit = println(local + param("cat=deprecation") + nonConstant("x"))
