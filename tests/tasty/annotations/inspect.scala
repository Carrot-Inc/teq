// The annotations scalac reads of a class's definitions (tests/tasty.sh, "The annotations"),
// reported as a warning at compile time: the class's own, each member's, its parameters' and type
// parameters', the inherited methods annotated `@org.junit.Test` (a reflective discovery), each
// annotation as its class and its arguments' values, so that scalac's own pickles and teq's can
// be compared whatever trees they spell a value with.
package inspect

import scala.quoted.*

object Inspect:
  inline def annotations[T]: Unit = ${ annotationsOf[T] }
  /** The module class of the object of the name, a file's top-level definitions' among them. */
  inline def moduleAnnotations(inline name: String): Unit = ${ moduleAnnotationsOf('name) }

  def annotationsOf[T: Type](using Quotes): Expr[Unit] =
    import quotes.reflect.*
    listing(TypeRepr.of[T].typeSymbol)

  def moduleAnnotationsOf(name: Expr[String])(using Quotes): Expr[Unit] =
    import quotes.reflect.*
    listing(Symbol.requiredModule(name.valueOrAbort).moduleClass)

  def listing(using Quotes)(cls: quotes.reflect.Symbol): Expr[Unit] =
    import quotes.reflect.*
    def typeName(t: TypeRepr): String = t.dealias.widen match
      case AppliedType(tycon, args) => s"${typeName(tycon)}[${args.map(typeName).mkString(", ")}]"
      case other => other.typeSymbol.fullName
    def constant(c: Constant): String = c match
      case LongConstant(v) => s"${v}L"
      case FloatConstant(v) => s"${v}f"
      case DoubleConstant(v) => s"${v}d"
      case CharConstant(v) => s"'$v'"
      case StringConstant(v) => s"\"$v\""
      case ClassOfConstant(t) => s"classOf[${typeName(t)}]"
      case other => String.valueOf(other.value)
    def args(t: Term): List[Term] = t match
      case Apply(f, as) => args(f) ++ as
      case TypeApply(f, _) => args(f)
      case _ => Nil
    def head(t: Term): Term = t match
      case Apply(f, _) => head(f)
      case TypeApply(f, _) => head(f)
      case other => other
    def value(t: Term): String = t match
      case Inlined(_, _, e) => value(e)
      case Typed(e, _) => value(e)
      case Block(Nil, e) => value(e)
      case Literal(c) => constant(c)
      case NamedArg(n, v) => s"$n = ${value(v)}"
      case Repeated(elems, _) => elems.map(value).mkString(", ")
      case _ =>
        val h = head(t)
        val sym = h.symbol
        if sym.name == "classOf" then
          val TypeApply(_, List(tpt)) = t: @unchecked
          s"classOf[${typeName(tpt.tpe)}]"
        else if sym.name.contains("$default$") then "<default>"
        else if sym.isClassConstructor then s"new ${typeName(t.tpe)}(${args(t).map(value).mkString(", ")})"
        else if sym.name == "apply" && sym.owner.fullName == "scala.Array$" then
          s"Array(${args(t).filter(a => !(a.tpe <:< TypeRepr.of[scala.reflect.ClassTag[?]])).map(value).mkString(", ")})"
        else if sym.isDefDef then s"${sym.fullName}(${args(t).map(value).mkString(", ")})"
        else if t.isExpr || sym.isTerm then sym.fullName
        else t.show
    def annots(s: Symbol): String =
      s.annotations.filterNot(_.tpe.typeSymbol.fullName.startsWith("scala.annotation.internal.")).map(value).mkString(" ")
    val lines = List.newBuilder[String]
    lines += s"class ${cls.fullName}: ${annots(cls)}"
    for p <- cls.typeMembers.filter(_.isTypeParam) do lines += s"  type parameter ${p.name}: ${annots(p)}"
    for clause <- cls.primaryConstructor.paramSymss; p <- clause do lines += s"  constructor parameter ${p.name}: ${annots(p)}"
    val constructors = cls.declarations.filter(s => s.isClassConstructor && s != cls.primaryConstructor)
    val members = (cls.declaredFields ++ cls.declaredMethods ++ cls.declaredTypes ++ constructors).distinct.filterNot(_ == cls.primaryConstructor).sortBy(m => (m.name, m.signature.paramSigs.mkString(",")))
    for m <- members do
      lines += s"  ${if m.isTerm then "term" else "type"} ${m.name}: ${annots(m)}"
      for clause <- m.paramSymss; p <- clause do
        val a = annots(p)
        if a.nonEmpty then lines += s"    parameter ${p.name}: $a"
    val tests = cls.methodMembers.filter(_.annotations.exists(_.tpe.typeSymbol.fullName == "org.junit.Test")).map(_.name).sorted
    lines += s"  @Test methods: ${tests.mkString(", ")}"
    report.warning(lines.result().mkString("\n"))
    '{ () }
