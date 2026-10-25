// The representation oracle's probes of the reflection (tests/tasty/representation/README.md, "The reflection's
// probes"): each probe the review passes over dc2a3dfec, 3362b74bf and 4f95a8370 ran against teq's reflection, as a
// call of its own whose record is what the API answers, the types written by the observer's own walk
// (`Observe.walk`, no printer) and reported as the observer reports (`OBSERVE`, read by Oracle.scala for scalac's runs,
// by oracle.py for teq's). A probe asks what its pass asked and decides nothing.
package observe

import scala.quoted.*

object Probe:
  /** The probe `name`'s record, `probe:<name> <answer>`, over the type `T` its subject is (an owner or a reference). */
  inline def run[T](inline name: String): String = ${ runImpl[T]('name) }

  def runImpl[T: Type](name: Expr[String])(using Quotes): Expr[String] =
    val n = name.valueOrAbort
    val answer = Observe.record(s"probe:$n", answerOf[T](n))
    Observe.hits = Set.empty
    quotes.reflect.report.warning("OBSERVE\n" + answer)
    Expr(n)

  def answerOf[T: Type](n: String)(using Quotes): String =
    import quotes.reflect.*
    val t = TypeRepr.of[T]
    def ty(u: TypeRepr): String = Observe.walk(u, Nil)
    def widened(u: TypeRepr): TypeRepr = u.widenTermRefByName
    def field(s: String): TypeRepr = t.typeSymbol.fieldMember(s).termRef
    def method(s: String): TypeRepr = t.typeSymbol.methodMember(s).head.termRef
    def params(u: TypeRepr): String = u match
      case mt: MethodType => Observe.list(mt.paramTypes.map(ty))
      case other => s"NotMethod(${ty(other)})"
    // A method of the probe's own, its information the type given (`Symbol.newMethod`), its reference widened.
    def made(name: String, info: TypeRepr): String =
      ty(widened(Symbol.newMethod(Symbol.spliceOwner, name, info).termRef))
    def runtimeChecked: Term = '{ new scala.annotation.internal.RuntimeChecked() }.asTerm
    n match
      // dc2a3dfec, finding 3: a val of `this.type` widens to its ThisType; a nested object's reference to its module
      // class's TypeRef (`Inner$`).
      case "this-type" => ty(widened(field("x")))
      case "module-class" => ty(widened(t))
      // 3362b74bf, finding 1: a method's and a polymorphic method's information; an annotated val's AnnotatedType; a
      // reference by name to an overloaded method.
      case "method-info" => ty(widened(method("f")))
      case "poly-info" => ty(widened(method("g")))
      case "annotated-val" => ty(widened(field("a")))
      case "overload" =>
        val r = TermRef(t, "over")
        val w = widened(r)
        s"${ty(w)} same=${w == r}"
      // 4f95a8370, finding 1: a dependent result's ParamRef, a repeated parameter's `<repeated>`, an inline parameter's
      // InlineParam.
      case "dependent-result" => ty(widened(method("dep")))
      case "repeated" => params(widened(method("rep")))
      case "inline-param" => params(widened(method("inl")))
      // Finding 2: a parameter reference read from one widening keeps its binder after the reference is widened again.
      case "binder-retained" =>
        widened(method("g")) match
          case g1: PolyType =>
            g1.resType match
              case m1: MethodType =>
                m1.resType match
                  case r1: ParamRef =>
                    val before = r1.binder == g1
                    val g2 = widened(method("g"))
                    s"before=$before after=${r1.binder == g1} same=${g1 == g2}"
                  case other => s"NotParamRef(${ty(other)})"
              case other => s"NotMethod(${ty(other)})"
          case other => s"NotPoly(${ty(other)})"
      // Finding 3: a polymorphic method's bound seen from its prefix (`C[String]`), through the extractor and through
      // the accessor.
      case "adapted-bound" =>
        widened(TermRef(field("c"), "h")) match
          case PolyType(_, bounds, _) => Observe.list(bounds.map(ty))
          case other => s"NotPoly(${ty(other)})"
      case "adapted-bound-accessor" =>
        widened(TermRef(field("c"), "h")) match
          case p: PolyType => Observe.list(p.paramBounds.map(ty))
          case other => s"NotPoly(${ty(other)})"
      // Finding 4: a member found through an intersection, its information both arms' joint one.
      case "joint" => ty(widened(TermRef(field("ab"), "f")))
      // Finding 5: an annotated type's annotation is one tree, read twice.
      case "annotation-tree" =>
        widened(field("a")) match
          case a: AnnotatedType => s"same=${a.annotation == a.annotation}"
          case other => s"NotAnnotated(${ty(other)})"
      // Finding 6, which the pass gave no input: `widenExpr` removes one ExprType, the outer one, and none under an
      // annotation; a method made with that information, through the API, is the input.
      case "by-name-nested" => made("byNameNested", ByNameType(ByNameType(TypeRepr.of[Int])))
      case "by-name-annotated" => made("byNameAnnotated", AnnotatedType(ByNameType(TypeRepr.of[Int]), runtimeChecked))
      case other => report.errorAndAbort(s"no probe $other")
