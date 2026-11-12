// Class constants of each kind `getClassBType` tells apart, `Nothing` and `Null` (scala-library's `Nothing$` and
// `Null$`), a primitive and `Unit` (the boxed class's `TYPE`), a reference class (by `ldc`): as a top-level inline val
// (each read the constant), a file module's constant final val and an object's (each getter answering it), beside
// the `classOf` trees, which take the same class literal (an array's: tests/cases/class_constant_arrays).
package classkinds

inline val nothingI = classOf[Nothing]
inline val nullI = classOf[Null]
inline val intI = classOf[Int]
inline val unitI = classOf[Unit]
inline val stringI = classOf[String]

final val nothingF = classOf[Nothing]
final val nullF = classOf[Null]
final val intF = classOf[Int]
final val unitF = classOf[Unit]
final val stringF = classOf[String]

object Holder:
  final val nothingO = classOf[Nothing]
  final val nullO = classOf[Null]
  final val intO = classOf[Int]
  final val unitO = classOf[Unit]
  final val stringO = classOf[String]

@main def run(): Unit =
  println(s"$nothingI $nullI $intI $unitI $stringI")
  println(s"$nothingF $nullF $intF $unitF $stringF")
  println(s"${Holder.nothingO} ${Holder.nullO} ${Holder.intO} ${Holder.unitO} ${Holder.stringO}")
  println(s"${classOf[Nothing]} ${classOf[Null]} ${classOf[Int]} ${classOf[Unit]} ${classOf[String]}")
  println(nothingI == classOf[Nothing] && nullF == classOf[Null] && Holder.unitO == classOf[Unit])
