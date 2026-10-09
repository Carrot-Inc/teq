package scala.reflect

/** A `ClassTag[T]` carries the class of `T`. The compiler supplies one for a concrete `T` where a
  * `using` parameter asks for it, as scalac does; `classOf[T]` is the class it carries.
  */
final class ClassTag[T](val runtimeClass: Class[?]):
  @jvm("rt $0 $1:I rtcall newArrayOfTag(Lscala/reflect/ClassTag;I)Ljava/lang/Object; cast_result")
  @jvmLink("$0 $1:I invokeinterface scala/reflect/ClassTag.newArray(I)Ljava/lang/Object; cast_result")
  def newArray(len: Int): Array[T] = unsafeCast(scala.newArray[Any](len, zero))
  private def zero: Any = if runtimeClass == null then null else runtimeClass.getName match
    case "int" | "short" | "byte" => 0
    case "long" => 0L
    case "double" | "float" => 0.0
    case "boolean" => false
    case "char" => '\u0000'
    case "void" => ()
    case _ => null
  /** The extractor a type pattern over an abstract `T` goes through (`Typer::tag_pattern`):
    * scala-library's tests `runtimeClass.isInstance(x)`, which here is each primitive's own
    * test (the interpreter tells a `Byte` from an `Int`; JavaScript has one number), `null`
    * never an instance, and an array's tag the one array class's test (arrays have no kind
    * outside the JVM). The JVM links scala-library's body.
    */
  def unapply(x: Any): Option[T] =
    val fits: Boolean = runtimeClass.getName match
      case "int" => x.isInstanceOf[Int]
      case "short" => x.isInstanceOf[Short]
      case "byte" => x.isInstanceOf[Byte]
      case "long" => x.isInstanceOf[Long]
      case "double" => x.isInstanceOf[Double]
      case "float" => x.isInstanceOf[Float]
      case "boolean" => x.isInstanceOf[Boolean]
      case "char" => x.isInstanceOf[Char]
      case "void" => x.isInstanceOf[Unit]
      case "java.lang.Object" => x != null
      case n if n.startsWith("[") => x.isInstanceOf[Array[?]]
      case _ => x != null && runtimeClass.isInstance(x)
    if fits then Some(unsafeCast(x)) else None
  @jvm("rt $0 rtcall wrappedTag(Lscala/reflect/ClassTag;)Lscala/reflect/ClassTag;")
  @jvmLink("$0 invokeinterface scala/reflect/ClassTag.wrap()Lscala/reflect/ClassTag;")
  def wrap: ClassTag[Array[T]] = new ClassTag(classOf[Array[T]])
  override def equals(that: Any): Boolean = that match
    case c: ClassTag[?] => c.runtimeClass.getName == runtimeClass.getName
    case _ => false
  override def hashCode: Int = runtimeClass.getName.hashCode
  override def toString: String = runtimeClass.getName match
    case "int" => "Int"
    case "long" => "Long"
    case "double" => "Double"
    case "float" => "Float"
    case "boolean" => "Boolean"
    case "char" => "Char"
    case "byte" => "Byte"
    case "short" => "Short"
    case "void" => "Unit"
    case "java.lang.Object" => "Object"
    case n => n

/** `scala.reflect.classTag[T]`, the evidence itself, as a library body names it. */
def classTag[T](using ct: ClassTag[T]): ClassTag[T] = ct

object ClassTag:
  def apply[T](runtimeClass: Class[?]): ClassTag[T] = new ClassTag(runtimeClass)
  val Byte: ClassTag[Byte] = ClassTag(classOf[Byte])
  val Short: ClassTag[Short] = ClassTag(classOf[Short])
  val Char: ClassTag[Char] = ClassTag(classOf[Char])
  val Int: ClassTag[Int] = ClassTag(classOf[Int])
  val Long: ClassTag[Long] = ClassTag(classOf[Long])
  val Float: ClassTag[Float] = ClassTag(classOf[Float])
  val Double: ClassTag[Double] = ClassTag(classOf[Double])
  val Boolean: ClassTag[Boolean] = ClassTag(classOf[Boolean])
  val Unit: ClassTag[Unit] = ClassTag(classOf[Unit])
  val Any: ClassTag[Any] = ClassTag(classOf[Any])
  val Object: ClassTag[AnyRef] = ClassTag(classOf[AnyRef])
  val AnyVal: ClassTag[AnyVal] = ClassTag(classOf[AnyVal])
  val AnyRef: ClassTag[AnyRef] = ClassTag(classOf[AnyRef])
