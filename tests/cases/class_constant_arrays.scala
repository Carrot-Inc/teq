//> using platform jvm
// (JavaScript left out: teq's JavaScript output gives every array class as `[Ljava.lang.Object;`, a `classOf` tree's
// too, on master too.)
// Class constants of array classes, `ldc` of the array's class: of a primitive, of `Unit` (its elements boxed), of a
// reference class, nested, as a top-level inline val, a file module's constant final val and an object's, beside the
// `classOf` trees.
package classarrays

inline val intsI = classOf[Array[Int]]
inline val unitsI = classOf[Array[Unit]]
inline val stringsI = classOf[Array[String]]
inline val nestedI = classOf[Array[Array[Long]]]

final val intsF = classOf[Array[Int]]
final val unitsF = classOf[Array[Unit]]

object Holder:
  final val stringsO = classOf[Array[String]]
  final val nestedO = classOf[Array[Array[Long]]]

@main def run(): Unit =
  println(s"$intsI $unitsI $stringsI $nestedI")
  println(s"$intsF $unitsF ${Holder.stringsO} ${Holder.nestedO}")
  println(s"${classOf[Array[Int]]} ${classOf[Array[Unit]]} ${classOf[Array[String]]} ${classOf[Array[Array[Long]]]}")
