package scala

// The type only singleton types conform to: literal types and stable paths (`x.type`).
sealed abstract class Singleton

// Accepted and ignored, like the members of scala.annotation.
final class unchecked
final class specialized

// scala-library's annotations, read by name and otherwise ignored; their constructors, parameter
// names, defaults and meta annotations are scala-library's, which the products' pickles name.
@getter @setter @beanGetter @beanSetter @field
class deprecated(message: String = "", since: String = "")
@getter @setter @beanGetter @beanSetter
class deprecatedOverriding(message: String = "", since: String = "")
@getter @setter @beanGetter @beanSetter
final class deprecatedInheritance(message: String = "", since: String = "")
@param
class deprecatedName(name: String = "", since: String = "")
class main
final class throws[T <: Throwable](cause: String = ""):
  def this(clazz: Class[T]) = this("")
class SerialVersionUID(value: Long)
@field
final class transient
@field
final class volatile
class inline
final class noinline
class native
