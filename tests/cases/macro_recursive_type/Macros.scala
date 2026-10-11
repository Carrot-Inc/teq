package mrec
import scala.quoted.*

trait Box

object Recursive:
  inline def of[A]: String = ${ ofImpl[A] }

  def ofImpl[A: Type](using q: Quotes): Expr[String] =
    import q.reflect.*
    def show(t: TypeRepr): String = t match
      case r: RecursiveType => "rec(" + show(r.underlying) + ")"
      case RecursiveThis(_) => "self"
      case r: Refinement => show(r.parent) + " { " + r.name + " " + info(r.info) + " }"
      case TypeRef(RecursiveThis(_), name) => "self." + name
      case TypeRef(_, name) => name
      case other => "?"
    def info(t: TypeRepr): String = t match
      case TypeBounds(lo, hi) if lo =:= hi => "= " + show(hi)
      case TypeBounds(lo, hi) => ">: " + show(lo) + " <: " + show(hi)
      case ByNameType(u) => ": => " + show(u)
      case m: MethodType => m.paramNames.mkString("(", ", ", ")") + info(m.resType)
      case other => ": " + show(other)
    // The member a refinement names through its self: a `TypeRef` over the `RecursiveThis`,
    // whose binder is the recursive type itself.
    def selfBound(rt: RecursiveType): Boolean =
      def go(t: TypeRepr): Boolean = t match
        case r: Refinement => go(r.parent) || go(r.info)
        case TypeBounds(lo, hi) => go(lo) || go(hi)
        case m: MethodType => go(m.resType)
        case TypeRef(pre, _) => go(pre)
        case RecursiveThis(b) => b =:= rt
        case _ => false
      go(rt.underlying)
    val read = TypeRepr.of[A].dealias match
      case rt: RecursiveType => show(rt) + " bound " + selfBound(rt)
      case other => "not recursive: " + show(other)
    val built = RecursiveType(rt => Refinement(TypeRepr.of[Box], "T", TypeBounds(TypeRepr.of[Nothing], TypeRepr.of[Any])))
    val shape = built match
      case RecursiveType(u) => "built " + show(u) + " " + (built.recThis match { case RecursiveThis(b) => b =:= built })
    Expr(read + " | " + shape)

  // An equal recursive type built twice is the one type, its self the one it has (dotty's
  // interned `RecType`, its cached `recThis`).
  inline def stable: String = ${ stableImpl }

  def stableImpl(using q: Quotes): Expr[String] =
    import q.reflect.*
    val r1 = RecursiveType(_ => TypeRepr.of[Box])
    val t1 = r1.recThis
    val r2 = RecursiveType(_ => TypeRepr.of[Box])
    Expr((r1 =:= r2).toString + " " + (t1 =:= r1.recThis).toString + " " + (t1 =:= r2.recThis).toString)
