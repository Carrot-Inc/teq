package scala.quoted

// Quotes and splices run in the compiler's interpreter (src/interp/quoted.rs): an `Expr[T]` is a
// typed tree of the program, a `Type[T]` a type of the compiler, a `Symbol` an entry of its
// symbol table. The members marked @js("$quoted") are builtins of the interpreter keyed by their
// qualified name; the others run as written.

abstract class Expr[+T]

abstract class Type[T]:
  type Underlying = T

trait Quotes:
  val reflect: Reflect.type = Reflect

  extension [T](self: Expr[T])
    @js("$quoted") def show: String
    @js("$quoted") def matches(that: Expr[Any]): Boolean
    def value(using from: FromExpr[T]): Option[T] = from.unapply(self)(using this)
    def valueOrAbort(using from: FromExpr[T]): T = from.unapply(self)(using this) match
      case Some(v) => v
      case None => reflect.report.errorAndAbort("Expected a known value. \n\nThe value of: " + self.show + "\ncould not be extracted using " + from.toString, self)
    def valueOrError(using FromExpr[T]): T = valueOrAbort

  // Declared as scala-library declares them, without a type parameter of the receiver: a
  // pickled body calls them as `quotes.asExprOf(e)[X]`.
  extension (self: Expr[Any])
    @js("$quoted") def isExprOf[X](using Type[X]): Boolean
    def asExprOf[X](using Type[X]): Expr[X] =
      if isExprOf[X] then self.asInstanceOf[Expr[X]]
      else reflect.report.errorAndAbort("Expr cast exception: " + self.show + "\nof type: " + reflect.Printer.TypeReprCode.show(reflect.asTerm(self).tpe) + "\ndid not conform to type: " + reflect.Printer.TypeReprCode.show(reflect.TypeRepr.of[X]) + "\n", self)

  extension [T](self: Type[T])
    def show: String = reflect.Printer.TypeReprCode.show(reflect.TypeRepr.of[T](using self))

object QuotesImpl extends Quotes:
  // scalac's compiler context, which izumi-reflect reads reflectively (`qctx.ctx`) to pass back
  // to the internal methods the interpreter answers (`TypeRef.underlying(ctx)`): a token.
  def ctx: AnyRef = this

def quotes(using q: Quotes): q.type = q

object Expr:
  def apply[T](x: T)(using to: ToExpr[T])(using Quotes): Expr[T] = to.apply(x)
  def unapply[T](x: Expr[T])(using from: FromExpr[T])(using Quotes): Option[T] = from.unapply(x)
  def betaReduce[T](expr: Expr[T])(using Quotes): Expr[T] =
    import quotes.reflect.*
    Term.betaReduce(expr.asTerm) match
      case Some(e) => e.asExpr.asInstanceOf[Expr[T]]
      case None => expr
  def block[T](statements: List[Expr[Any]], expr: Expr[T])(using Quotes): Expr[T] =
    import quotes.reflect.*
    (Block(statements.map(s => s.asTerm), expr.asTerm).asExpr).asInstanceOf[Expr[T]]
  def ofSeq[T](xs: Seq[Expr[T]])(using Type[T])(using Quotes): Expr[Seq[T]] = Varargs(xs)
  def ofList[T](xs: Seq[Expr[T]])(using Type[T])(using Quotes): Expr[List[T]] =
    if xs.isEmpty then '{ Nil } else '{ List(${ Varargs(xs) }*) }
  def ofTupleFromSeq(seq: Seq[Expr[Any]])(using Quotes): Expr[Tuple] =
    import quotes.reflect.*
    val elems = seq.toList.map(e => e.asTerm)
    if elems.isEmpty then '{ Tuple() }
    else Reflect.tupleOf(elems)
  def ofTuple[T <: Tuple: Tuple.IsMappedBy[Expr]: Type](tup: T)(using Quotes): Expr[Tuple.InverseMap[T, Expr]] =
    val elems = (tup.asInstanceOf[Product].productIterator.toSeq).asInstanceOf[Seq[Expr[Any]]]
    (ofTupleFromSeq(elems)).asInstanceOf[Expr[Tuple.InverseMap[T, Expr]]]
  def summon[T](using Type[T])(using Quotes): Option[Expr[T]] =
    import quotes.reflect.*
    Implicits.search(TypeRepr.of[T]) match
      case s: ImplicitSearchSuccess => Some(s.tree.asExpr.asInstanceOf[Expr[T]])
      case _ => None
  def summonIgnoring[T](using Type[T])(using q: Quotes)(ignored: q.reflect.Symbol*): Option[Expr[T]] = summon[T]

object Type:
  def of[T](using Quotes)(using t: Type[T]): Type[T] = t
  def show[T](using Type[T])(using Quotes): String = quotes.reflect.Printer.TypeReprCode.show(quotes.reflect.TypeRepr.of[T])
  def valueOfConstant[T](using Type[T])(using Quotes): Option[T] =
    import quotes.reflect.*
    TypeRepr.of[T].widenTermRefByName.dealias match
      case ConstantType(c) => Some(c.value.asInstanceOf[T])
      case _ => None
  def valueOfTuple[T <: Tuple](using Type[T])(using Quotes): Option[T] =
    import quotes.reflect.*
    def rec(t: TypeRepr): Option[Tuple] = t.widenTermRefByName.dealias match
      case AppliedType(fn, args) if defn.isTupleClass(fn.typeSymbol) =>
        args.foldRight(Option[Tuple](EmptyTuple)) {
          case (_, None) => None
          case (ConstantType(c), Some(acc)) => Some(c.value *: acc)
          case _ => None
        }
      case t if t.typeSymbol == defn.EmptyTupleClass => Some(EmptyTuple)
      case _ => None
    (rec(TypeRepr.of[T])).asInstanceOf[Option[T]]

trait ToExpr[T]:
  def apply(x: T)(using Quotes): Expr[T]

object ToExpr:
  import Reflect.*
  private def lit[T](c: Any): Expr[T] = (Literal(Constant(c)).asExpr).asInstanceOf[Expr[T]]
  given BooleanToExpr[T <: Boolean]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given ByteToExpr[T <: Byte]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given ShortToExpr[T <: Short]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given IntToExpr[T <: Int]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given LongToExpr[T <: Long]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given FloatToExpr[T <: Float]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given DoubleToExpr[T <: Double]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given CharToExpr[T <: Char]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given StringToExpr[T <: String]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given UnitToExpr: ToExpr[Unit] with
    def apply(x: Unit)(using Quotes): Expr[Unit] = lit(x)
  given ClassToExpr[T <: Class[?]]: ToExpr[T] with
    def apply(x: T)(using Quotes): Expr[T] = lit(x)
  given NoneToExpr: ToExpr[None.type] with
    def apply(x: None.type)(using Quotes): Expr[None.type] = '{ None }
  given NilToExpr: ToExpr[Nil.type] with
    def apply(x: Nil.type)(using Quotes): Expr[Nil.type] = '{ Nil }
  given SomeToExpr[T: Type: ToExpr]: ToExpr[Some[T]] with
    def apply(x: Some[T])(using Quotes): Expr[Some[T]] = '{ Some[T](${ Expr(x.get) }) }
  given OptionToExpr[T: Type: ToExpr]: ToExpr[Option[T]] with
    def apply(x: Option[T])(using Quotes): Expr[Option[T]] = x match
      case Some(v) => '{ Some[T](${ Expr(v) }) }
      case None => '{ None }
  given LeftToExpr[L: Type: ToExpr, R: Type]: ToExpr[Left[L, R]] with
    def apply(x: Left[L, R])(using Quotes): Expr[Left[L, R]] = '{ Left[L, R](${ Expr(x.value) }) }
  given RightToExpr[L: Type, R: Type: ToExpr]: ToExpr[Right[L, R]] with
    def apply(x: Right[L, R])(using Quotes): Expr[Right[L, R]] = '{ Right[L, R](${ Expr(x.value) }) }
  given EitherToExpr[L: Type: ToExpr, R: Type: ToExpr]: ToExpr[Either[L, R]] with
    def apply(x: Either[L, R])(using Quotes): Expr[Either[L, R]] = x match
      case Left(v) => '{ Left[L, R](${ Expr(v) }) }
      case Right(v) => '{ Right[L, R](${ Expr(v) }) }
  given ListToExpr[T: Type: ToExpr]: ToExpr[List[T]] with
    def apply(xs: List[T])(using Quotes): Expr[List[T]] = Expr.ofList(xs.map(x => Expr(x)))
  given SeqToExpr[T: Type: ToExpr]: ToExpr[Seq[T]] with
    def apply(xs: Seq[T])(using Quotes): Expr[Seq[T]] = Varargs(xs.map(x => Expr(x)))
  given ArrayToExpr[T: Type: ToExpr: scala.reflect.ClassTag]: ToExpr[Array[T]] with
    def apply(xs: Array[T])(using Quotes): Expr[Array[T]] = '{ Array[T](${ Varargs(xs.toList.map(x => Expr(x))) }*) }
  given SetToExpr[T: Type: ToExpr]: ToExpr[Set[T]] with
    def apply(xs: Set[T])(using Quotes): Expr[Set[T]] = '{ Set[T](${ Varargs(xs.toList.map(x => Expr(x))) }*) }
  given MapToExpr[K: Type: ToExpr, V: Type: ToExpr]: ToExpr[Map[K, V]] with
    def apply(m: Map[K, V])(using Quotes): Expr[Map[K, V]] = '{ Map[K, V](${ Varargs(m.toList.map(p => Expr(p))) }*) }
  given Tuple1ToExpr[T1: Type: ToExpr]: ToExpr[Tuple1[T1]] with
    def apply(t: Tuple1[T1])(using Quotes): Expr[Tuple1[T1]] = '{ Tuple1[T1](${ Expr(t._1) }) }
  given Tuple2ToExpr[T1: Type: ToExpr, T2: Type: ToExpr]: ToExpr[Tuple2[T1, T2]] with
    def apply(t: Tuple2[T1, T2])(using Quotes): Expr[Tuple2[T1, T2]] = '{ (${ Expr(t._1) }, ${ Expr(t._2) }) }
  given Tuple3ToExpr[T1: Type: ToExpr, T2: Type: ToExpr, T3: Type: ToExpr]: ToExpr[Tuple3[T1, T2, T3]] with
    def apply(t: Tuple3[T1, T2, T3])(using Quotes): Expr[Tuple3[T1, T2, T3]] = '{ (${ Expr(t._1) }, ${ Expr(t._2) }, ${ Expr(t._3) }) }
  given Tuple4ToExpr[T1: Type: ToExpr, T2: Type: ToExpr, T3: Type: ToExpr, T4: Type: ToExpr]: ToExpr[Tuple4[T1, T2, T3, T4]] with
    def apply(t: Tuple4[T1, T2, T3, T4])(using Quotes): Expr[Tuple4[T1, T2, T3, T4]] = '{ (${ Expr(t._1) }, ${ Expr(t._2) }, ${ Expr(t._3) }, ${ Expr(t._4) }) }
  given StringContextToExpr: ToExpr[StringContext] with
    def apply(sc: StringContext)(using Quotes): Expr[StringContext] = '{ StringContext(${ Varargs(sc.parts.map(p => Expr(p))) }*) }

trait FromExpr[T]:
  def unapply(x: Expr[T])(using Quotes): Option[T]

object FromExpr:
  import Reflect.*
  private def constant[T](x: Expr[Any])(using Quotes)(pick: Any => Option[T]): Option[T] =
    def rec(t: Term): Option[T] = t match
      case Literal(c) => pick(c.value)
      case Block(Nil, e) => rec(e)
      case Typed(e, _) => rec(e)
      case Inlined(_, Nil, e) => rec(e)
      case _ => None
    rec(x.asTerm)
  given BooleanFromExpr[T <: Boolean]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Boolean => Some(v.asInstanceOf[T]); case _ => None }
  given ByteFromExpr[T <: Byte]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Byte => Some(v.asInstanceOf[T]); case _ => None }
  given ShortFromExpr[T <: Short]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Short => Some(v.asInstanceOf[T]); case _ => None }
  given IntFromExpr[T <: Int]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Int => Some(v.asInstanceOf[T]); case _ => None }
  given LongFromExpr[T <: Long]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Long => Some(v.asInstanceOf[T]); case _ => None }
  given FloatFromExpr[T <: Float]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Float => Some(v.asInstanceOf[T]); case _ => None }
  given DoubleFromExpr[T <: Double]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Double => Some(v.asInstanceOf[T]); case _ => None }
  given CharFromExpr[T <: Char]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: Char => Some(v.asInstanceOf[T]); case _ => None }
  given StringFromExpr[T <: String]: FromExpr[T] with
    def unapply(x: Expr[T])(using Quotes): Option[T] = constant(x) { case v: String => Some(v.asInstanceOf[T]); case _ => None }
  given OptionFromExpr[T](using Type[T], FromExpr[T]): FromExpr[Option[T]] with
    def unapply(x: Expr[Option[T]])(using Quotes): Option[Option[T]] = x match
      case '{ Option[T](${ Expr(y) }) } => Some(Option(y))
      case '{ None } => Some(None)
      case '{ ${ Expr(y) }: Some[T] } => Some(y)
      case _ => None
  given NoneFromExpr: FromExpr[None.type] with
    def unapply(x: Expr[None.type])(using Quotes): Option[None.type] = x match
      case '{ None } => Some(None)
      case _ => None
  given SomeFromExpr[T](using Type[T], FromExpr[T]): FromExpr[Some[T]] with
    def unapply(x: Expr[Some[T]])(using Quotes): Option[Some[T]] = x match
      case '{ new Some[T](${ Expr(y) }) } => Some(Some(y))
      case '{ Some[T](${ Expr(y) }) } => Some(Some(y))
      case _ => None
  given NilFromExpr: FromExpr[Nil.type] with
    def unapply(x: Expr[Nil.type])(using Quotes): Option[Nil.type] = x match
      case '{ Nil } => Some(Nil)
      case _ => None
  given ListFromExpr[T](using Type[T], FromExpr[T]): FromExpr[List[T]] with
    def unapply(x: Expr[List[T]])(using Quotes): Option[List[T]] = x match
      case '{ List[T](${ Varargs(Exprs(elems)) }*) } => Some(elems.toList)
      case '{ List.empty[T] } => Some(Nil)
      case '{ Nil } => Some(Nil)
      case _ => None
  given SeqFromExpr[T](using Type[T], FromExpr[T]): FromExpr[Seq[T]] with
    def unapply(x: Expr[Seq[T]])(using Quotes): Option[Seq[T]] = x match
      case Varargs(Exprs(elems)) => Some(elems)
      case '{ Seq[T](${ Varargs(Exprs(elems)) }*) } => Some(elems)
      case '{ List[T](${ Varargs(Exprs(elems)) }*) } => Some(elems)
      case _ => None
  given StringContextFromExpr: FromExpr[StringContext] with
    def unapply(x: Expr[StringContext])(using Quotes): Option[StringContext] = x match
      case '{ new StringContext(${ Varargs(Exprs(parts)) }*) } => Some(StringContext(parts*))
      case '{ StringContext(${ Varargs(Exprs(parts)) }*) } => Some(StringContext(parts*))
      case _ => None
  given Tuple1FromExpr[T1](using Type[T1], FromExpr[T1]): FromExpr[Tuple1[T1]] with
    def unapply(x: Expr[Tuple1[T1]])(using Quotes): Option[Tuple1[T1]] = x match
      case '{ Tuple1[T1](${ Expr(a) }) } => Some(Tuple1(a))
      case _ => None
  given Tuple2FromExpr[T1, T2](using Type[T1], Type[T2], FromExpr[T1], FromExpr[T2]): FromExpr[Tuple2[T1, T2]] with
    def unapply(x: Expr[Tuple2[T1, T2]])(using Quotes): Option[Tuple2[T1, T2]] = x match
      case '{ (${ Expr(a) }: T1, ${ Expr(b) }: T2) } => Some((a, b))
      case _ => None
  given Tuple3FromExpr[T1, T2, T3](using Type[T1], Type[T2], Type[T3], FromExpr[T1], FromExpr[T2], FromExpr[T3]): FromExpr[Tuple3[T1, T2, T3]] with
    def unapply(x: Expr[Tuple3[T1, T2, T3]])(using Quotes): Option[Tuple3[T1, T2, T3]] = x match
      case '{ (${ Expr(a) }: T1, ${ Expr(b) }: T2, ${ Expr(c) }: T3) } => Some((a, b, c))
      case _ => None

object Varargs:
  def apply[T](xs: Seq[Expr[T]])(using Type[T])(using Quotes): Expr[Seq[T]] =
    import quotes.reflect.*
    (Repeated(xs.toList.map(x => x.asTerm), TypeTree.of[T]).asExpr).asInstanceOf[Expr[Seq[T]]]
  def unapply[T](expr: Expr[Seq[T]])(using Quotes): Option[Seq[Expr[T]]] =
    import quotes.reflect.*
    def rec(tree: Term): Option[Seq[Expr[T]]] = tree match
      case Repeated(elems, _) => Some(elems.map(x => x.asExpr.asInstanceOf[Expr[T]]))
      case Typed(e, _) => rec(e)
      case Block(Nil, e) => rec(e)
      case Inlined(_, Nil, e) => rec(e)
      case _ => None
    rec(expr.asTerm)

object Exprs:
  def unapply[T](exprs: Seq[Expr[T]])(using FromExpr[T])(using Quotes): Option[Seq[T]] =
    val out = scala.collection.mutable.ArrayBuffer.empty[T]
    var ok = true
    for e <- exprs do
      if ok then e.value match
        case Some(v) => out += v
        case None => ok = false
    if ok then Some(out.toList) else None

// A map over the terms of an expression: `transformChildren` walks one level with the
// reflect API's `TreeMap`, `transform` is what an implementation does at each term.
trait ExprMap:
  def transform[T](e: Expr[T])(using Type[T])(using Quotes): Expr[T]
  def transformChildren[T](e: Expr[T])(using Type[T])(using Quotes): Expr[T] =
    import quotes.reflect.*
    final class MapChildren:
      def transformStatement(tree: Statement)(owner: Symbol): Statement = tree match
        case tree: Term => transformTerm(tree, TypeRepr.of[Any])(owner)
        case tree: ValDef =>
          val rhs1 = tree.rhs.map(x => transformTerm(x, tree.tpt.tpe)(tree.symbol))
          ValDef.copy(tree)(tree.name, tree.tpt, rhs1)
        case tree: DefDef =>
          val rhs1 = tree.rhs.map(x => transformTerm(x, tree.returnTpt.tpe)(tree.symbol))
          DefDef.copy(tree)(tree.name, tree.paramss, tree.returnTpt, rhs1)
        case _ => tree
      def transformTerm(tree: Term, tpe: TypeRepr)(owner: Symbol): Term = tree match
        case Typed(expr, tpt) => Typed.copy(tree)(transformTerm(expr, tpt.tpe)(owner), tpt)
        case Block(stats, expr) => Block.copy(tree)(stats.map(s => transformStatement(s)(owner)), transformTerm(expr, tpe)(owner))
        case If(cond, thenp, elsep) =>
          If.copy(tree)(transformTerm(cond, TypeRepr.of[Boolean])(owner), transformTerm(thenp, tpe)(owner), transformTerm(elsep, tpe)(owner))
        case Apply(fun, args) =>
          val newArgs = args.map(a => transformTerm(a, a.tpe.widen)(owner))
          Apply.copy(tree)(fun, newArgs)
        case Match(selector, cases) =>
          Match.copy(tree)(transformTerm(selector, selector.tpe.widen)(owner), cases.map(c => CaseDef.copy(c)(c.pattern, c.guard, transformTerm(c.rhs, tpe)(owner))))
        case While(cond, body) => While.copy(tree)(transformTerm(cond, TypeRepr.of[Boolean])(owner), transformTerm(body, TypeRepr.of[Unit])(owner))
        case Return(expr, from) => Return.copy(tree)(transformTerm(expr, expr.tpe.widen)(owner), from)
        case Repeated(elems, elemtpt) => Repeated.copy(tree)(elems.map(e => transformTerm(e, elemtpt.tpe)(owner)), elemtpt)
        case Inlined(call, bindings, expansion) => Inlined.copy(tree)(call, bindings, transformTerm(expansion, tpe)(owner))
        case _ => tree
      def transformTermChildren(tree: Term, tpe: TypeRepr)(owner: Symbol): Term = tree match
        case Typed(expr, tpt) => Typed.copy(tree)(transformTerm(expr, tpt.tpe)(owner), tpt)
        case Block(stats, expr) => Block.copy(tree)(stats.map(s => transformStatement(s)(owner)), transformTerm(expr, tpe)(owner))
        case _ => transformTerm(tree, tpe)(owner)
    val term = e.asTerm
    (new MapChildren).transformTermChildren(term, TypeRepr.of[T])(Symbol.spliceOwner).asExprOf[T]

package runtime:
  // What `report.errorAndAbort` throws; the expander ends the expansion when it arrives.
  final class StopMacroExpansion extends Throwable
