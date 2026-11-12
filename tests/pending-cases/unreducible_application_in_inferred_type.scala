//> using options -Werror -source 3.0-migration
// teq: --werror --source 3.0-migration
// `Checking.checkAppliedTypesIn` (Checking.scala 145-172) over an inferred type, from PostTyper's
// `checkInferredWellFormed` (PostTyper.scala 281-289, only where no error was reported): `mk`'s inferred result
// `F[? <: Int]` is an unreducible application, E043 "... in inferred type F[? <: Int]" (`showInferred`, Checking.scala
// 55-60), a migration warning under 3.0-migration and so an error under -Werror: scalac 3.8.4 rejects the program
// ("No warnings can be incurred under -Werror"). The written one (`get`'s) is silenced by `@nowarn`.
// teq is to reject the program too.
import scala.annotation.nowarn
trait T[F[_]] {
  @nowarn def get: F[? <: Int]
}
object Main {
  def mk[F[_]](t: T[F]) = t.get
  def main(args: Array[String]): Unit = println("ok")
}
