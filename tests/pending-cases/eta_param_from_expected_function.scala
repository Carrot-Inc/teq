// Pinned for the queued part inlining-lila-folds.
// scalac accepts and prints the expected output.
// A method eta-expanded against a function type
// takes the expected parameter type for its
// parameter (`EtaExpansion.etaExpand` declares
// none; `Typer.typedFunctionValue` takes the
// formal where it is fully defined), so a dependent
// `BuildFrom[xs.type, T, C]` sees a `List`.
import scala.collection.BuildFrom
object Rnd:
  def rev[T, C](xs: IterableOnce[T])(using bf: BuildFrom[xs.type, T, C]): C =
    bf.fromSpecific(xs)(xs.iterator.toList.reverse)
case class P(n: Int)
def groups(puzzles: List[P], size: Int): List[List[P]] =
  puzzles.grouped(size).toList.take(3).map(Rnd.rev)
object Main:
  def main(args: Array[String]): Unit =
    println(groups(List(P(1), P(2), P(3), P(4)), 2))
