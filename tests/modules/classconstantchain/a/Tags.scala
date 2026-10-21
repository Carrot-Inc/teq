package cca

import scala.reflect.ClassTag

// A class constant of a type the build makes while typing the bodies (`classOf[Tagged[Left, Right]]`, a `Class[..]` no
// signature names): a whole build large enough to type on several workers makes that type in a worker's own store,
// and the class constant keeps it (`LitVal::Class`); its products are the module's built alone, `ClassTag.apply`'s
// argument a `Class` with no cast (the gate of `82464a1b7`'s API chain, a derived `Labelling`'s labels array).
final class Tagged[A, B]
final class Left
final class Right

object Tags:
  val pair: ClassTag[Tagged[Left, Right]] = ClassTag.apply(classOf[Tagged[Left, Right]])
  val made: Array[Tagged[Right, Left]] = scala.runtime.Arrays.newGenericArray(1)(using ClassTag.apply(classOf[Tagged[Right, Left]]))
  def name: String = s"${pair.runtimeClass.getSimpleName} ${made.length}"
