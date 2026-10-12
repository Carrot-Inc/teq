// A Scala class in a `java` package is Scala's: its `Array[T]` of a type parameter erases to `Object`, as scalac's, where
// a member of the std's stand-in for a JDK class erases as Java's (`Array[T]` to `Object[]`, dotty's
// `isGenericArrayElement` reading the parameter's `JavaDefined` provenance).
// abi: ScalaAPI#array ScalaAPI#arrays
package java.custom
class ScalaAPI:
  def array[T](x: Array[T]): Array[T] = x
  def arrays[T <: AnyRef](x: Array[T]): Array[T] = x
