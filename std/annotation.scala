package scala.annotation

// What an annotation class extends for scalac.
trait Annotation
trait StaticAnnotation extends Annotation
trait ConstantAnnotation extends StaticAnnotation

// Accepted and ignored; they exist so that imports of them resolve. Their constructors,
// parameter names, defaults and meta annotations are scala-library's, which the products'
// pickles name.
final class tailrec
@setter @getter
class unused(message: String):
  def this() = this("")
class nowarn(value: String = "")
final class targetName(name: String)
final class implicitNotFound(msg: String)
@getter
final class implicitAmbiguous(msg: String)
@setter @param @beanSetter @beanGetter @getter @field
final class static
final class experimental(message: String):
  def this() = this("")
final class publicInBinary
final class switch
final class threadUnsafe
final class varargs
@companionMethod @companionClass @beanSetter @beanGetter @setter @getter
final class compileTimeOnly(message: String)
final class unroll
@field @param
class constructorOnly

package unchecked:
  final class uncheckedVariance
  @field @getter
  final class uncheckedStable
  final class uncheckedCaptures

package meta:
  final class field
  final class getter
  final class setter
  final class param
  final class beanGetter
  final class beanSetter
  final class companionClass
  final class companionMethod
  final class companionObject
  final class languageFeature(feature: String, enableRequired: Boolean)
