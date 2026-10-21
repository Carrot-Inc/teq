// A binder's `@nowarn` covers the binder, from its annotations to its bounds (`Typer.completeAnnotations`'
// `registerNowarn` over the binder's `TypeDef`), whichever clause holds it; a binder without one warns.
import scala.annotation.nowarn
@deprecated("old", "1") class Old

type Covered = [@nowarn A <: Old] =>> Int
def kinded[F[@nowarn A <: Old]] = 1
val literal = [@nowarn A <: Old] => (a: A) => a
type Function = [@nowarn("cat=deprecation") A <: Old] => A => A
type Bare = [A <: Old] =>> Int
val bare = [A <: Old] => (a: A) => a

@main def run(): Unit = println(kinded[List])
