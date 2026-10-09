package caa

// An `Array[Int]` cast to `Array[AnyRef]` is checked (no primitive array is one of references; the
// JVM throws, the lean targets have no element kind), a generic union element erases its array to
// `Object` (`isGenericArrayElement`), and an inline expansion fixes the element.
object Lib:
  def primitive(x: Array[Int]): Unit = { x.asInstanceOf[Array[AnyRef]]; () }
  inline def primitiveInline(x: Array[Int]): Unit = { x.asInstanceOf[Array[AnyRef]]; () }
  def generic[T](x: Any): Unit = { x.asInstanceOf[Array[T | Int]]; () }
  inline def genericInline[T](x: Any): Unit = { x.asInstanceOf[Array[T | Int]]; () }
