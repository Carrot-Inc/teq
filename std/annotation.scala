package scala.annotation

// What an annotation class extends for scalac.
trait Annotation
trait StaticAnnotation extends Annotation
trait ConstantAnnotation extends StaticAnnotation

// Accepted and ignored; they exist so that imports of them resolve. Their constructors,
// parameter names, defaults and meta annotations are scala-library's, which the products'
// pickles name.
final class tailrec extends StaticAnnotation
@setter @getter
class unused(message: String) extends StaticAnnotation:
  def this() = this("")
class nowarn(value: String = "") extends ConstantAnnotation
final class targetName(name: String) extends StaticAnnotation
final class implicitNotFound(msg: String) extends ConstantAnnotation
@getter
final class implicitAmbiguous(msg: String) extends ConstantAnnotation
@setter @param @beanSetter @beanGetter @getter @field
final class static extends StaticAnnotation
final class experimental(message: String) extends StaticAnnotation:
  def this() = this("")
final class publicInBinary extends StaticAnnotation
final class switch extends StaticAnnotation
final class threadUnsafe extends StaticAnnotation
final class varargs extends StaticAnnotation
@companionMethod @companionClass @beanSetter @beanGetter @setter @getter
final class compileTimeOnly(message: String) extends StaticAnnotation
final class unroll extends StaticAnnotation
@field @param
class constructorOnly extends StaticAnnotation

// The annotation of `Predef.runtimeChecked`'s result, which a pattern's checks read on the
// scrutinee's type.
package internal:
  final class RuntimeChecked() extends Annotation

package unchecked:
  final class uncheckedVariance extends StaticAnnotation
  @field @getter
  final class uncheckedStable extends StaticAnnotation
  final class uncheckedCaptures extends StaticAnnotation

package meta:
  final class field extends StaticAnnotation
  final class getter extends StaticAnnotation
  final class setter extends StaticAnnotation
  final class param extends StaticAnnotation
  final class beanGetter extends StaticAnnotation
  final class beanSetter extends StaticAnnotation
  final class companionClass extends StaticAnnotation
  final class companionMethod extends StaticAnnotation
  final class companionObject extends StaticAnnotation
  final class languageFeature(feature: String, enableRequired: Boolean) extends StaticAnnotation
