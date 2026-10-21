// A binder's `@nowarn` reads its filter as `Annotations.argumentConstant` does, the cast stripped (`stripCast`) and the
// operand's type read, a block's type its last expression's: each silences the bound's deprecation under `-Werror`
// (the review of `b23cfe2a3`'s NowarnCast and NowarnBlockFold).
import scala.annotation.nowarn
@deprecated("old", "1") class Old

type Cast = [@nowarn("cat=deprecation".asInstanceOf[String]) A <: Old] =>> List[A]
type Block = [@nowarn({ println("side"); "cat=" + "deprecation" }) A <: Old] =>> List[A]

@main def run(): Unit = println("ok")
