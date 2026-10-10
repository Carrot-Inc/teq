package scala.reflect

/** scala-library's `ClassTag`, the class of `T` (`runtimeClass`): a trait a program may implement,
  * overriding `newArray`, `unapply`, `wrap` or the others too. The compiler supplies one for a
  * concrete `T` where a `using` parameter asks for it, as scalac does: `ClassTag.apply(classOf[T])`,
  * a primitive's tag for its class, or `ClassTag.Unit`, `Any` and `AnyVal`.
  */
trait ClassTag[T]:
  def runtimeClass: Class[?]
  // scala-library's bodies, each reading `runtimeClass` once: an array of the class (the one
  // array class's outside the JVM, its elements the zero of the class), a value of the class
  // (`isInstance`), an array's tag (the one array class), and the tag's equality, hash and name.
  @jvm("rt $0 $1:I rtcall newArrayOfTag(Lscala/reflect/ClassTag;I)Ljava/lang/Object; cast_result")
  @jvmLink("$0 $1:I invokeinterface scala/reflect/ClassTag.newArray(I)Ljava/lang/Object; cast_result")
  def newArray(len: Int): Array[T] = unsafeCast(scala.newArray[Any](len, ClassTag.zeroOf(runtimeClass)))
  /** The extractor a type pattern over an abstract `T` goes through (`Typer::tag_pattern`). */
  def unapply(x: Any): Option[T] = if runtimeClass.isInstance(x) then Some(unsafeCast(x)) else None
  @jvm("rt $0 rtcall wrappedTag(Lscala/reflect/ClassTag;)Lscala/reflect/ClassTag;")
  @jvmLink("$0 invokeinterface scala/reflect/ClassTag.wrap()Lscala/reflect/ClassTag;")
  def wrap: ClassTag[Array[T]] = ClassTag(ClassTag.arrayClassOf(runtimeClass))
  def canEqual(x: Any): Boolean = x.isInstanceOf[ClassTag[?]]
  override def equals(x: Any): Boolean = x.isInstanceOf[ClassTag[?]] && this.runtimeClass == x.asInstanceOf[ClassTag[?]].runtimeClass
  override def hashCode: Int = runtimeClass.##
  override def toString: String = ClassTag.prettyprint(runtimeClass)

/** `scala.reflect.classTag[T]`, the evidence itself, as a library body names it. */
def classTag[T](using ct: ClassTag[T]): ClassTag[T] = ct

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

object ClassTag:
  val Byte: ClassTag[Byte] = new PrimitiveClassTag(classOf[Byte], "Byte", _.isInstanceOf[Byte], 0.toByte)
  val Short: ClassTag[Short] = new PrimitiveClassTag(classOf[Short], "Short", _.isInstanceOf[Short], 0.toShort)
  val Char: ClassTag[Char] = new PrimitiveClassTag(classOf[Char], "Char", _.isInstanceOf[Char], '\u0000')
  val Int: ClassTag[Int] = new PrimitiveClassTag(classOf[Int], "Int", _.isInstanceOf[Int], 0)
  val Long: ClassTag[Long] = new PrimitiveClassTag(classOf[Long], "Long", _.isInstanceOf[Long], 0L)
  val Float: ClassTag[Float] = new PrimitiveClassTag(classOf[Float], "Float", _.isInstanceOf[Float], 0.0f)
  val Double: ClassTag[Double] = new PrimitiveClassTag(classOf[Double], "Double", _.isInstanceOf[Double], 0.0)
  val Boolean: ClassTag[Boolean] = new PrimitiveClassTag(classOf[Boolean], "Boolean", _.isInstanceOf[Boolean], false)
  val Unit: ClassTag[Unit] = new PrimitiveClassTag(classOf[Unit], "Unit", _.isInstanceOf[Unit], ())
  val Any: ClassTag[Any] = new PhantomClassTag(classOf[Object], "Any")
  val Object: ClassTag[Object] = new PhantomClassTag(classOf[Object], "Object")
  val AnyVal: ClassTag[AnyVal] = new PhantomClassTag(classOf[Object], "AnyVal")
  val AnyRef: ClassTag[AnyRef] = Object

  /** scala-library's `apply`: a primitive's and `Object`'s class give their tags, any other a
    * `GenericClassTag`.
    */
  def apply[T](runtimeClass: Class[?]): ClassTag[T] = unsafeCast(runtimeClass.getName match
    case "byte" => Byte
    case "short" => Short
    case "char" => Char
    case "int" => Int
    case "long" => Long
    case "float" => Float
    case "double" => Double
    case "boolean" => Boolean
    case "void" => Unit
    case "java.lang.Object" => Object
    case _ => new GenericClassTag(runtimeClass))

  def unapply[T](ctag: ClassTag[T]): Option[Class[?]] = Some(ctag.runtimeClass)

  // The element a new array of the class holds.
  private[reflect] def zeroOf(c: Class[?]): Any = c.getName match
    case "int" | "short" | "byte" => 0
    case "long" => 0L
    case "double" | "float" => 0.0
    case "boolean" => false
    case "char" => '\u0000'
    case "void" => ()
    case _ => null

  // scala-library's `arrayClass`: the one array class outside the JVM, whatever the element.
  private[reflect] def arrayClassOf(c: Class[?]): Class[?] = classOf[Array[Any]]

  // scala-library's `prettyprint`: an array's class as `Array[element]`.
  private[reflect] def prettyprint(c: Class[?]): String =
    if c.isArray then "Array[" + prettyprint(c.getComponentType) + "]" else c.getName
