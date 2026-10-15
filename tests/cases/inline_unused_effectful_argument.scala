// An unused inline parameter whose argument constructs a class that runs an initialiser keeps its
// binding: `Inliner.isElideableExpr` takes a construction as pure only where the class runs none
// (`TreeInfo.isPureApply`), and the program's `asInstanceOf` is never pure, whatever the parent.
import scala.deriving.Mirror
inline def ignore(m: Mirror.Product): Int = 7
@main def run(): Unit =
  println(ignore((new Mirror.Product {
    type MirroredType = Int
    type MirroredMonoType = Int
    type MirroredElemTypes = EmptyTuple
    type MirroredElemLabels = EmptyTuple
    type MirroredLabel = "C"
    println("effect")
    def fromProduct(p: Product): Int = 1
  }: AnyRef).asInstanceOf[Mirror.Product]))
