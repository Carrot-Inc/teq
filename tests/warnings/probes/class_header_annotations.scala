// The annotations on the types of a class's header use their imports: a parameter's type, a type
// parameter's bound, a parent.
package headers
object Lib {
  class Ann extends scala.annotation.StaticAnnotation
  class Bnd extends scala.annotation.StaticAnnotation
  class Par extends scala.annotation.StaticAnnotation
  class Unused extends scala.annotation.StaticAnnotation
  trait Base
}
import Lib.Ann
import Lib.Bnd
import Lib.Par
import Lib.Unused
import Lib.Base
class C(x: Int @Ann)
class D[T <: AnyRef @Bnd]
class E extends (Base @Par)
