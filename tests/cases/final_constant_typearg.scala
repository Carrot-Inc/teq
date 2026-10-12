// A final constant's singleton (`C.x.type` of `final val x = 1`) is the same type as the
// constant `1`, an invariant type argument included: dotty compares a path whose underlying
// type is a singleton with that singleton (`TypeComparer.comparePaths`, `compareAtoms`).
class Box[A]
object C:
  final val x = 1
  final val y: 2 = 2
@main def run() =
  val a: Box[C.x.type] = new Box[1]
  val b: Box[1] = new Box[C.x.type]
  val c: Box[C.y.type] = new Box[2]
  println(a != null && b != null && c != null)
