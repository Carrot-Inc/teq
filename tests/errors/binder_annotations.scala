// A binder's annotation, a higher-kinded parameter's own parameter's or a type lambda's, is a modifier the typer
// types (dotty's `TypeDef.withMods`, `Namer.addAnnotations`, `Typer.completeAnnotations`): an unknown class is
// scalac 3.8.4's E006, an argument of another type its E007; a well-typed one passes.
// expect: 9:17: error: type DoesNotExist not found
// expect: 10:20: error: type mismatch: found "wrong", required Int
// expect: 12:12: error: type Nope not found
// expect: 13:25: error: type mismatch: found "w", required Int
class ann(x: Int) extends scala.annotation.StaticAnnotation
trait Higher[F[@DoesNotExist A]]
trait Wrong[F[@ann("wrong") A]]
trait Right[F[@ann(1) A]]
type L = [@Nope X] =>> List[X]
object O { def m[G[@ann("w") Y]] = 1 }
