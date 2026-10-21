package scala

// The type only singleton types conform to: literal types and stable paths (`x.type`).
sealed abstract class Singleton

// Accepted and ignored, like the members of scala.annotation.
final class unchecked extends scala.annotation.Annotation
final class specialized extends scala.annotation.StaticAnnotation

// scala-library's annotations, read by name and otherwise ignored; their constructors, parameter
// names, defaults and meta annotations are scala-library's, which the products' pickles name.
@getter @setter @beanGetter @beanSetter @field
class deprecated(message: String = "", since: String = "") extends scala.annotation.ConstantAnnotation
@getter @setter @beanGetter @beanSetter
class deprecatedOverriding(message: String = "", since: String = "") extends scala.annotation.ConstantAnnotation
@getter @setter @beanGetter @beanSetter
final class deprecatedInheritance(message: String = "", since: String = "") extends scala.annotation.ConstantAnnotation
@param
class deprecatedName(name: String = "", since: String = "") extends scala.annotation.StaticAnnotation
class main extends scala.annotation.Annotation
final class throws[T <: Throwable](cause: String = "") extends scala.annotation.StaticAnnotation:
  def this(clazz: Class[T]) = this("")
class SerialVersionUID(value: Long) extends scala.annotation.ConstantAnnotation
@field
final class transient extends scala.annotation.StaticAnnotation
@field
final class volatile extends scala.annotation.StaticAnnotation
class inline extends scala.annotation.StaticAnnotation
final class noinline extends scala.annotation.StaticAnnotation
class native extends scala.annotation.StaticAnnotation
