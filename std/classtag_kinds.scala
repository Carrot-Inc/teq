// The tags' classes and the helpers of `ClassTag`'s bodies (`classtag.scala`), apart so that only a
// program that makes a tag enters them.
package scala.reflect

/** scala-library's `ClassTag.GenericClassTag`, the tag `ClassTag.apply` makes of a class that is no
  * primitive's, `Object`'s, `Nothing`'s or `Null`'s.
  */
final class GenericClassTag[T](val runtimeClass: Class[?]) extends ClassTag[T]

/** scala-library's `AnyValManifest` (`ManifestFactory.IntManifest` and the others): the tag of a
  * primitive class, named by its Scala type, its values by their type test, its arrays filled with
  * its zero, equal to itself alone.
  */
final class PrimitiveClassTag[T](val runtimeClass: Class[?], name: String, test: Any => Boolean, zero: Any) extends ClassTag[T]:
  override def newArray(len: Int): Array[T] = unsafeCast(scala.newArray[Any](len, zero))
  override def unapply(x: Any): Option[T] = if test(x) then Some(unsafeCast(x)) else None
  override def canEqual(other: Any): Boolean = other.isInstanceOf[PrimitiveClassTag[?]]
  override def equals(that: Any): Boolean = this eq that.asInstanceOf[AnyRef]
  override def hashCode: Int = System.identityHashCode(this)
  override def toString: String = name

/** scala-library's `PhantomManifest` (`Any`, `Object`, `AnyVal`; the std has no class of `Nothing`
  * or `Null`, and so no tag of theirs): a class that stands for the type, named by the type, equal to
  * itself alone.
  */
final class PhantomClassTag[T](val runtimeClass: Class[?], name: String) extends ClassTag[T]:
  override def equals(that: Any): Boolean = this eq that.asInstanceOf[AnyRef]
  override def hashCode: Int = System.identityHashCode(this)
  override def toString: String = name

// The element a new array of the class holds.
private[reflect] def zeroOf(c: Class[?]): Any = c.getName match
  case "int" | "short" | "byte" => 0
  case "long" => 0L
  case "double" | "float" => 0.0
  case "boolean" => false
  case "char" => '\u0000'
  case "void" => ()
  case _ => null

// scala-library's `arrayClass`: the one array class outside the JVM, whatever the element, as an
// array value is of it.
private[reflect] def arrayClassOf(c: Class[?]): Class[?] = classOf[Array[Any]]

// scala-library's `prettyprint`: an array's class as `Array[element]`.
private[reflect] def prettyprint(c: Class[?]): String =
  if c.isArray then "Array[" + prettyprint(c.getComponentType) + "]" else c.getName
