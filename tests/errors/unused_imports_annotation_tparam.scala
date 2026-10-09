// A parameter's annotation sees its definition's type parameters.
package annotation_param
object Lib { class T }
class Ann[A] extends scala.annotation.StaticAnnotation
import Lib.T
class C[T](@Ann[T] x: Int)

// teq: --werror --wunused imports
// expect: unused_imports_annotation_tparam.scala:5:12: warning: unused import
