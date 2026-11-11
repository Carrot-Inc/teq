import scala.quoted.*

// A macro that annotates the val it makes with a Java annotation built through the reflection API: the
// application of the annotation's constructor, which dotty makes up of its elements
// (ClassfileParser.addAnnotationConstructor), to its one element's array.
object Marked:
  inline def marked[A](inline a: A): A = ${ impl('a) }

  def impl[A: Type](a: Expr[A])(using Quotes): Expr[A] =
    import quotes.reflect.*
    val annotationSymbol = TypeRepr.of[java.lang.SuppressWarnings].typeSymbol
    val annotation = Apply(Select(New(TypeIdent(annotationSymbol)), annotationSymbol.primaryConstructor), List(Expr(Array("all")).asTerm))
    val name = Symbol.newVal(Symbol.spliceOwner, "marked", AnnotatedType(TypeRepr.of[A], annotation), Flags.EmptyFlags, Symbol.noSymbol)
    Block(List(ValDef(name, Some(a.asTerm.changeOwner(name)))), Ref(name)).asExprOf[A]

  inline def parameters: String = ${ parametersImpl }

  def parametersImpl(using Quotes): Expr[String] =
    import quotes.reflect.*
    val constructor = TypeRepr.of[java.lang.Deprecated].typeSymbol.primaryConstructor
    Expr(constructor.paramSymss.map(_.map(_.name).mkString("(", ", ", ")")).mkString)

  inline def owned: Boolean = ${ ownedImpl }

  def ownedImpl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    val constructor = TypeRepr.of[java.lang.Deprecated].typeSymbol.primaryConstructor
    Expr(constructor.paramSymss.flatten.forall(_.owner == constructor))

  inline def synthetic: Boolean = ${ syntheticImpl }

  def syntheticImpl(using Quotes): Expr[Boolean] =
    import quotes.reflect.*
    val constructor = TypeRepr.of[java.lang.SuppressWarnings].typeSymbol.primaryConstructor
    Expr(constructor.flags.is(Flags.Synthetic | Flags.JavaDefined) && constructor.paramSymss.flatten.forall(_.flags.is(Flags.Synthetic | Flags.Param)))
