// jars: scala-library izumi-reflect-jvm izumi-reflect-boopickle-jvm
// std: scala-library
//> using dep dev.zio::izumi-reflect:3.0.9
// An opaque type seen from outside its scope: izumi's `Tag` macro takes the closest class from
// `baseClasses`, which are its bound's (`Any`, so `Object`); its `ClassTag` and an array of it are
// the underlying type's (`Int`, an `int[]`), the class literal being the erasure's.
package infra

opaque type Aid = Int
object Aid:
  def authorize(v: Int): Aid = v
  extension (a: Aid) def value: Int = a
