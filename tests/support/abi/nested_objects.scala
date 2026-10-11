// An object nested in an object is a module val of it, a term member that is no method, so a public static final
// field of its module class (dotty's `BCodeSkelBuilder.addClassFields`), which nothing assigns, the nested object
// read through its own `MODULE$` (master wrote none): a written object's, an enum's companion's, and the companion
// the compiler makes for a case class. Its flags are the module term's (`javaFieldFlags`): a private object's field is
// private, a qualified private one's public.
// abi: Owner$#Nested Owner$#Other Owner$#Point Owner$#Color Owner$#weird$minusname Owner$#Hidden Owner$#Scoped
package nested
object Owner:
  object Nested:
    def x = 1
  object Other { val y = 2 }
  case class Point(a: Int)
  enum Color { case Red }
  object `weird-name` { def z = 3 }
  private object Hidden { val h = 4 }
  private[nested] object Scoped { val s = 5 }
  def total: Int = Hidden.h + Scoped.s
