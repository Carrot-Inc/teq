// `AnyKind` (`Definitions.AnyKindClass`, a synthetic class of `scala`),
// the bound of a parameter of any kind, which `hasSameKindAs` takes
// as the same kind as any other (`isAnyKind`): a part of its own.
import scala.annotation.StaticAnnotation
class annA[F <: AnyKind] extends StaticAnnotation
class annS[F <: scala.AnyKind] extends StaticAnnotation
type H10 = [@annA[[X] =>> A] A] =>> Int
type H13 = [@annS[[X] =>> B] B] =>> Int
trait Kinded[F <: AnyKind]
object K1 extends Kinded[List]
object K2 extends Kinded[Int]

@main def run(): Unit =
  println(s"${K1.getClass.getSimpleName} ${K2.getClass.getSimpleName}")
