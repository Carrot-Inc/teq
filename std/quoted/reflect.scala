package scala.quoted

// `quotes.reflect`: the reflection API over the compiler's trees, types and symbols. The node
// classes are never instantiated: a `Tree` is a typed tree of the program that the interpreter
// holds as a value of its own, a `TypeRepr` a type of the compiler, a `Symbol` an entry of its
// symbol table, and a type test on one of these classes asks the interpreter which kind the
// value is. The `XModule` objects hold the constructors and extractors, the `XMethods` objects
// the extension methods, as scala-library's `Quotes` does, so that a library's macro body,
// which names them through its `Quotes` instance, and a program's `import quotes.reflect.*`
// both find them.
object Reflect:
  extension (expr: Expr[Any])
    @js("$quoted") def asTerm: Term
  @js("$quoted") def tupleOf(elems: List[Term]): Expr[Tuple]

  // ---- trees ----

  class Tree
  class PackageClause extends Tree
  class Statement extends Tree
  class Import extends Statement
  class Export extends Statement
  class Definition extends Statement
  class ClassDef extends Definition
  class ValOrDefDef extends Definition
  class DefDef extends ValOrDefDef
  class ValDef extends ValOrDefDef
  class TypeDef extends Definition
  class Term extends Statement
  class Ref extends Term
  class Ident extends Ref
  class Wildcard extends Ident
  class Select extends Ref
  class Literal extends Term
  class This extends Term
  class New extends Term
  class NamedArg extends Term
  class Apply extends Term
  class TypeApply extends Term
  class Super extends Term
  trait TypedOrTest extends Tree
  class Typed extends Term, TypedOrTest
  class Assign extends Term
  class Block extends Term
  class Closure extends Term
  class If extends Term
  class Match extends Term
  class SummonFrom extends Term
  class Try extends Term
  class Return extends Term
  class Repeated extends Term
  class Inlined extends Term
  class SelectOuter extends Term
  class While extends Term
  class TypeTree extends Tree
  class Inferred extends TypeTree
  class TypeIdent extends TypeTree
  class TypeSelect extends TypeTree
  class TypeProjection extends TypeTree
  class Singleton extends TypeTree
  class Refined extends TypeTree
  class Applied extends TypeTree
  class Annotated extends TypeTree
  class MatchTypeTree extends TypeTree
  class ByName extends TypeTree
  class LambdaTypeTree extends TypeTree
  class TypeBind extends TypeTree
  class TypeBlock extends TypeTree
  class TypeBoundsTree extends Tree
  class WildcardTypeTree extends Tree
  class CaseDef extends Tree
  class TypeCaseDef extends Tree
  class Bind extends Tree
  class Unapply extends Tree
  class Alternatives extends Tree
  // A parameter clause is the list of its parameters at run time, as in scalac, whose
  // `ParamClause` is an abstract type: a `List(ValDef(..))` pattern matches a term clause.
  trait ParamClause
  trait TermParamClause extends ParamClause
  trait TypeParamClause extends ParamClause

  object TreeMethods:
    extension (self: Tree)
      @js("$quoted") def pos: Position
      @js("$quoted") def symbol: Symbol
      def show(using Printer[Tree]): String = summon[Printer[Tree]].show(self)
      // A term of a value type: a method selected without its arguments is none.
      def isExpr: Boolean = self match
        case t: Term =>
          t.tpe.widen match
            case _: MethodType | _: PolyType => false
            case _ => true
        case _ => false
      def asExpr: Expr[Any] = self match
        case t: Term if self.isExpr => Reflect.asExprOf(t)
        case _: Term => throw new Exception("Expected an expression. This is a partially applied Term. Try eta-expanding the term first.")
        case _ => throw new Exception("Expected a Term but was: " + self.show(using Printer.TreeStructure))
      // One using clause, as scala-library declares it: a pickled body passes one.
      def asExprOf[T](using Type[T]): Expr[T] =
        given Quotes = QuotesImpl
        self.asExpr.asExprOf[T]
    extension [ThisTree <: Tree](self: ThisTree)
      @js("$quoted") def changeOwner(newOwner: Symbol): ThisTree
  export TreeMethods.*
  @js("$quoted") def asExprOf(t: Term): Expr[Any]

  object PackageClause:
    @js("$quoted") def copy(original: Tree)(pid: Ref, stats: List[Tree]): PackageClause
    @js("$quoted") def unapply(tree: PackageClause): (Ref, List[Tree])
  object PackageClauseTypeTest:
    def unapply(x: Tree): Option[PackageClause] = if x.isInstanceOf[PackageClause] then Some(x.asInstanceOf[PackageClause]) else None

  object StatementTypeTest:
    def unapply(x: Tree): Option[Statement] = if x.isInstanceOf[Statement] then Some(x.asInstanceOf[Statement]) else None
  object DefinitionTypeTest:
    def unapply(x: Tree): Option[Definition] = if x.isInstanceOf[Definition] then Some(x.asInstanceOf[Definition]) else None
  object DefinitionMethods:
    extension (self: Definition)
      @js("$quoted") def name: String
  export DefinitionMethods.*

  object ClassDef:
    @js("$quoted") def apply(cls: Symbol, parents: List[Tree], body: List[Statement]): ClassDef
    @js("$quoted") def copy(original: Tree)(name: String, constr: DefDef, parents: List[Tree], selfOpt: Option[ValDef], body: List[Statement]): ClassDef
    @js("$quoted") def unapply(cdef: ClassDef): (String, DefDef, List[Tree], Option[ValDef], List[Statement])
  object ClassDefTypeTest:
    def unapply(x: Tree): Option[ClassDef] = if x.isInstanceOf[ClassDef] then Some(x.asInstanceOf[ClassDef]) else None
  object ClassDefMethods:
    extension (self: ClassDef)
      @js("$quoted") def constructor: DefDef
      @js("$quoted") def parents: List[Tree]
      @js("$quoted") def self: Option[ValDef]
      @js("$quoted") def body: List[Statement]
  export ClassDefMethods.*

  object ValOrDefDefTypeTest:
    def unapply(x: Tree): Option[ValOrDefDef] = if x.isInstanceOf[ValOrDefDef] then Some(x.asInstanceOf[ValOrDefDef]) else None
  object ValOrDefDefMethods:
    extension (self: ValOrDefDef)
      @js("$quoted") def tpt: TypeTree
      @js("$quoted") def rhs: Option[Term]
  export ValOrDefDefMethods.*

  object DefDef:
    @js("$quoted") def apply(symbol: Symbol, rhsFn: List[List[Tree]] => Option[Term]): DefDef
    @js("$quoted") def copy(original: Tree)(name: String, paramss: List[ParamClause], tpt: TypeTree, rhs: Option[Term]): DefDef
    @js("$quoted") def unapply(ddef: DefDef): (String, List[ParamClause], TypeTree, Option[Term])
  object DefDefTypeTest:
    def unapply(x: Tree): Option[DefDef] = if x.isInstanceOf[DefDef] then Some(x.asInstanceOf[DefDef]) else None
  object DefDefMethods:
    extension (self: DefDef)
      @js("$quoted") def paramss: List[ParamClause]
      def leadingTypeParams: List[TypeDef] = self.paramss match
        case (tc: TypeParamClause) :: _ => tc.params
        case _ => Nil
      def trailingParamss: List[ParamClause] = self.paramss match
        case (_: TypeParamClause) :: rest => rest
        case all => all
      def termParamss: List[TermParamClause] = self.paramss.collect { case tc: TermParamClause => tc }
      @js("$quoted") def returnTpt: TypeTree
      @js("$quoted") def rhs: Option[Term]
  export DefDefMethods.*

  object ValDef:
    @js("$quoted") def apply(symbol: Symbol, rhs: Option[Term]): ValDef
    @js("$quoted") def copy(original: Tree)(name: String, tpt: TypeTree, rhs: Option[Term]): ValDef
    @js("$quoted") def unapply(vdef: ValDef): (String, TypeTree, Option[Term])
    def let(owner: Symbol, name: String, rhs: Term)(body: Ref => Term): Term =
      val sym = Symbol.newVal(owner, name, rhs.tpe.widen, Flags.EmptyFlags, Symbol.noSymbol)
      Block(List(ValDef(sym, Some(rhs))), body(Ref(sym)))
    def let(owner: Symbol, rhs: Term)(body: Ref => Term): Term = let(owner, "x", rhs)(body)
    def let(owner: Symbol, terms: List[Term])(body: List[Ref] => Term): Term =
      val syms = terms.map(t => Symbol.newVal(owner, "x", t.tpe.widen, Flags.EmptyFlags, Symbol.noSymbol))
      Block(syms.zip(terms).map((s, t) => ValDef(s, Some(t))), body(syms.map(s => Ref(s))))
  object ValDefTypeTest:
    def unapply(x: Tree): Option[ValDef] = if x.isInstanceOf[ValDef] then Some(x.asInstanceOf[ValDef]) else None
  object ValDefMethods:
    extension (self: ValDef)
      @js("$quoted") def tpt: TypeTree
      @js("$quoted") def rhs: Option[Term]
      def tptOf: TypeTree = self.tpt
  export ValDefMethods.*

  object TypeDef:
    @js("$quoted") def apply(symbol: Symbol): TypeDef
    @js("$quoted") def copy(original: Tree)(name: String, rhs: Tree): TypeDef
    @js("$quoted") def unapply(tdef: TypeDef): (String, Tree)
  object TypeDefTypeTest:
    def unapply(x: Tree): Option[TypeDef] = if x.isInstanceOf[TypeDef] then Some(x.asInstanceOf[TypeDef]) else None
  object TypeDefMethods:
    extension (self: TypeDef)
      @js("$quoted") def rhs: Tree
  export TypeDefMethods.*

  object Term:
    @js("$quoted") def betaReduce(term: Term): Option[Term]
  object TermTypeTest:
    def unapply(x: Tree): Option[Term] = if x.isInstanceOf[Term] then Some(x.asInstanceOf[Term]) else None
  object TermMethods:
    extension (self: Term)
      @js("$quoted") def tpe: TypeRepr
      @js("$quoted") def underlyingArgument: Term
      @js("$quoted") def underlying: Term
      @js("$quoted") def etaExpand(owner: Symbol): Term
      def appliedTo(arg: Term): Term = self.appliedToArgs(arg :: Nil)
      def appliedTo(arg: Term, args: Term*): Term = self.appliedToArgs(arg :: args.toList)
      def appliedToArgs(args: List[Term]): Apply = Apply(self, args)
      def appliedToArgss(argss: List[List[Term]]): Term = argss.foldLeft(self)((acc, args) => Apply(acc, args))
      def appliedToNone: Apply = self.appliedToArgs(Nil)
      def appliedToType(targ: TypeRepr): Term = self.appliedToTypes(targ :: Nil)
      def appliedToTypes(targs: List[TypeRepr]): Term = self.appliedToTypeTrees(targs.map(t => Inferred(t)))
      def appliedToTypeTrees(targs: List[TypeTree]): Term = if targs.isEmpty then self else TypeApply(self, targs)
      def select(sym: Symbol): Select = Select(self, sym)
  export TermMethods.*

  object Ref:
    @js("$quoted") def term(tp: TermRef): Ref
    @js("$quoted") def apply(sym: Symbol): Ref
  object RefTypeTest:
    def unapply(x: Tree): Option[Ref] = if x.isInstanceOf[Ref] then Some(x.asInstanceOf[Ref]) else None

  object Ident:
    def apply(tmref: TermRef): Term = Ref.term(tmref)
    @js("$quoted") def copy(original: Tree)(name: String): Ident
    @js("$quoted") def unapply(tree: Ident): Some[String]
  object IdentTypeTest:
    def unapply(x: Tree): Option[Ident] = if x.isInstanceOf[Ident] then Some(x.asInstanceOf[Ident]) else None
  object IdentMethods:
    extension (self: Ident)
      @js("$quoted") def name: String
  export IdentMethods.*

  object Wildcard:
    @js("$quoted") def apply(): Wildcard
    def unapply(pattern: Wildcard): Boolean = true
  object WildcardTypeTest:
    def unapply(x: Tree): Option[Wildcard] = if x.isInstanceOf[Wildcard] then Some(x.asInstanceOf[Wildcard]) else None

  object Select:
    @js("$quoted") def apply(qualifier: Term, symbol: Symbol): Select
    @js("$quoted") def unique(qualifier: Term, name: String): Select
    @js("$quoted") def overloaded(qualifier: Term, name: String, targs: List[TypeRepr], args: List[Term]): Term
    @js("$quoted") def overloaded(qualifier: Term, name: String, targs: List[TypeRepr], args: List[Term], returnType: TypeRepr): Term
    @js("$quoted") def copy(original: Tree)(qualifier: Term, name: String): Select
    @js("$quoted") def unapply(x: Select): (Term, String)
  object SelectTypeTest:
    def unapply(x: Tree): Option[Select] = if x.isInstanceOf[Select] then Some(x.asInstanceOf[Select]) else None
  object SelectMethods:
    extension (self: Select)
      @js("$quoted") def qualifier: Term
      @js("$quoted") def name: String
      @js("$quoted") def signature: Option[Signature]
  export SelectMethods.*

  object Literal:
    @js("$quoted") def apply(constant: Constant): Literal
    def copy(original: Tree)(constant: Constant): Literal = Literal(constant)
    @js("$quoted") def unapply(x: Literal): Some[Constant]
  object LiteralTypeTest:
    def unapply(x: Tree): Option[Literal] = if x.isInstanceOf[Literal] then Some(x.asInstanceOf[Literal]) else None
  object LiteralMethods:
    extension (self: Literal)
      @js("$quoted") def constant: Constant
  export LiteralMethods.*

  object This:
    @js("$quoted") def apply(cls: Symbol): This
    @js("$quoted") def copy(original: Tree)(qual: Option[String]): This
    @js("$quoted") def unapply(x: This): Some[Option[String]]
  object ThisTypeTest:
    def unapply(x: Tree): Option[This] = if x.isInstanceOf[This] then Some(x.asInstanceOf[This]) else None
  object ThisMethods:
    extension (self: This)
      @js("$quoted") def id: Option[String]
  export ThisMethods.*

  object New:
    @js("$quoted") def apply(tpt: TypeTree): New
    def copy(original: Tree)(tpt: TypeTree): New = New(tpt)
    @js("$quoted") def unapply(x: New): Some[TypeTree]
  object NewTypeTest:
    def unapply(x: Tree): Option[New] = if x.isInstanceOf[New] then Some(x.asInstanceOf[New]) else None
  object NewMethods:
    extension (self: New)
      @js("$quoted") def tpt: TypeTree
  export NewMethods.*

  object NamedArg:
    @js("$quoted") def apply(name: String, arg: Term): NamedArg
    def copy(original: Tree)(name: String, arg: Term): NamedArg = NamedArg(name, arg)
    @js("$quoted") def unapply(x: NamedArg): (String, Term)
  object NamedArgTypeTest:
    def unapply(x: Tree): Option[NamedArg] = if x.isInstanceOf[NamedArg] then Some(x.asInstanceOf[NamedArg]) else None
  object NamedArgMethods:
    extension (self: NamedArg)
      @js("$quoted") def name: String
      @js("$quoted") def value: Term
  export NamedArgMethods.*

  object Apply:
    @js("$quoted") def apply(fun: Term, args: List[Term]): Apply
    def copy(original: Tree)(fun: Term, args: List[Term]): Apply = Apply(fun, args)
    @js("$quoted") def unapply(x: Apply): (Term, List[Term])
  object ApplyTypeTest:
    def unapply(x: Tree): Option[Apply] = if x.isInstanceOf[Apply] then Some(x.asInstanceOf[Apply]) else None
  object ApplyMethods:
    extension (self: Apply)
      @js("$quoted") def fun: Term
      @js("$quoted") def args: List[Term]
  export ApplyMethods.*

  object TypeApply:
    @js("$quoted") def apply(fun: Term, args: List[TypeTree]): TypeApply
    def copy(original: Tree)(fun: Term, args: List[TypeTree]): TypeApply = TypeApply(fun, args)
    @js("$quoted") def unapply(x: TypeApply): (Term, List[TypeTree])
  object TypeApplyTypeTest:
    def unapply(x: Tree): Option[TypeApply] = if x.isInstanceOf[TypeApply] then Some(x.asInstanceOf[TypeApply]) else None
  object TypeApplyMethods:
    extension (self: TypeApply)
      @js("$quoted") def fun: Term
      @js("$quoted") def args: List[TypeTree]
  export TypeApplyMethods.*

  object Super:
    @js("$quoted") def apply(qual: Term, mix: Option[String]): Super
    def copy(original: Tree)(qual: Term, mix: Option[String]): Super = Super(qual, mix)
    @js("$quoted") def unapply(x: Super): (Term, Option[String])
  object SuperTypeTest:
    def unapply(x: Tree): Option[Super] = if x.isInstanceOf[Super] then Some(x.asInstanceOf[Super]) else None
  object SuperMethods:
    extension (self: Super)
      @js("$quoted") def qualifier: Term
      @js("$quoted") def id: Option[String]
      @js("$quoted") def idPos: Position
  export SuperMethods.*

  object Typed:
    @js("$quoted") def apply(expr: Term, tpt: TypeTree): Typed
    def copy(original: Tree)(expr: Term, tpt: TypeTree): Typed = Typed(expr, tpt)
    @js("$quoted") def unapply(x: Typed): (Term, TypeTree)
  object TypedTypeTest:
    def unapply(x: Tree): Option[Typed] = if x.isInstanceOf[Typed] then Some(x.asInstanceOf[Typed]) else None
  object TypedMethods:
    extension (self: Typed)
      @js("$quoted") def expr: Term
      @js("$quoted") def tpt: TypeTree
  export TypedMethods.*

  object TypedOrTest:
    @js("$quoted") def apply(expr: Tree, tpt: TypeTree): TypedOrTest
    def copy(original: Tree)(expr: Tree, tpt: TypeTree): TypedOrTest = TypedOrTest(expr, tpt)
    @js("$quoted") def unapply(x: TypedOrTest): (Tree, TypeTree)
  object TypedOrTestTypeTest:
    def unapply(x: Tree): Option[TypedOrTest] = if x.isInstanceOf[TypedOrTest] then Some(x.asInstanceOf[TypedOrTest]) else None
  object TypedOrTestMethods:
    extension (self: TypedOrTest)
      @js("$quoted") def tree: Tree
      @js("$quoted") def tpt: TypeTree
  export TypedOrTestMethods.*

  object Assign:
    @js("$quoted") def apply(lhs: Term, rhs: Term): Assign
    def copy(original: Tree)(lhs: Term, rhs: Term): Assign = Assign(lhs, rhs)
    @js("$quoted") def unapply(x: Assign): (Term, Term)
  object AssignTypeTest:
    def unapply(x: Tree): Option[Assign] = if x.isInstanceOf[Assign] then Some(x.asInstanceOf[Assign]) else None
  object AssignMethods:
    extension (self: Assign)
      @js("$quoted") def lhs: Term
      @js("$quoted") def rhs: Term
  export AssignMethods.*

  object Block:
    @js("$quoted") def apply(stats: List[Statement], expr: Term): Block
    def copy(original: Tree)(stats: List[Statement], expr: Term): Block = Block(stats, expr)
    @js("$quoted") def unapply(x: Block): (List[Statement], Term)
  object BlockTypeTest:
    def unapply(x: Tree): Option[Block] = if x.isInstanceOf[Block] then Some(x.asInstanceOf[Block]) else None
  object BlockMethods:
    extension (self: Block)
      @js("$quoted") def statements: List[Statement]
      @js("$quoted") def expr: Term
  export BlockMethods.*

  object Closure:
    @js("$quoted") def apply(meth: Term, tpe: Option[TypeRepr]): Closure
    def copy(original: Tree)(meth: Tree, tpe: Option[TypeRepr]): Closure = Closure(meth.asInstanceOf[Term], tpe)
    @js("$quoted") def unapply(x: Closure): (Term, Option[TypeRepr])
  object ClosureTypeTest:
    def unapply(x: Tree): Option[Closure] = if x.isInstanceOf[Closure] then Some(x.asInstanceOf[Closure]) else None
  object ClosureMethods:
    extension (self: Closure)
      @js("$quoted") def meth: Term
      @js("$quoted") def tpeOpt: Option[TypeRepr]
  export ClosureMethods.*

  object Lambda:
    @js("$quoted") def apply(owner: Symbol, tpe: MethodType, rhsFn: (Symbol, List[Tree]) => Tree): Block
    @js("$quoted") def unapply(tree: Block): Option[(List[ValDef], Term)]

  object If:
    @js("$quoted") def apply(cond: Term, thenp: Term, elsep: Term): If
    def copy(original: Tree)(cond: Term, thenp: Term, elsep: Term): If = If(cond, thenp, elsep)
    @js("$quoted") def unapply(tree: If): (Term, Term, Term)
  object IfTypeTest:
    def unapply(x: Tree): Option[If] = if x.isInstanceOf[If] then Some(x.asInstanceOf[If]) else None
  object IfMethods:
    extension (self: If)
      @js("$quoted") def cond: Term
      @js("$quoted") def thenp: Term
      @js("$quoted") def elsep: Term
      def isInline: Boolean = false
  export IfMethods.*

  object Match:
    @js("$quoted") def apply(selector: Term, cases: List[CaseDef]): Match
    def copy(original: Tree)(selector: Term, cases: List[CaseDef]): Match = Match(selector, cases)
    @js("$quoted") def unapply(x: Match): (Term, List[CaseDef])
  object MatchTypeTest:
    def unapply(x: Tree): Option[Match] = if x.isInstanceOf[Match] then Some(x.asInstanceOf[Match]) else None
  object MatchMethods:
    extension (self: Match)
      @js("$quoted") def scrutinee: Term
      @js("$quoted") def cases: List[CaseDef]
      def isInline: Boolean = false
  export MatchMethods.*

  object SummonFrom:
    @js("$quoted") def apply(cases: List[CaseDef]): SummonFrom
    def copy(original: Tree)(cases: List[CaseDef]): SummonFrom = SummonFrom(cases)
    @js("$quoted") def unapply(x: SummonFrom): Some[List[CaseDef]]
  object SummonFromTypeTest:
    def unapply(x: Tree): Option[SummonFrom] = if x.isInstanceOf[SummonFrom] then Some(x.asInstanceOf[SummonFrom]) else None
  object SummonFromMethods:
    extension (self: SummonFrom)
      @js("$quoted") def cases: List[CaseDef]
  export SummonFromMethods.*

  object Try:
    @js("$quoted") def apply(expr: Term, cases: List[CaseDef], finalizer: Option[Term]): Try
    def copy(original: Tree)(expr: Term, cases: List[CaseDef], finalizer: Option[Term]): Try = Try(expr, cases, finalizer)
    @js("$quoted") def unapply(x: Try): (Term, List[CaseDef], Option[Term])
  object TryTypeTest:
    def unapply(x: Tree): Option[Try] = if x.isInstanceOf[Try] then Some(x.asInstanceOf[Try]) else None
  object TryMethods:
    extension (self: Try)
      @js("$quoted") def body: Term
      @js("$quoted") def cases: List[CaseDef]
      @js("$quoted") def finalizer: Option[Term]
  export TryMethods.*

  object Return:
    @js("$quoted") def apply(expr: Term, from: Symbol): Return
    def copy(original: Tree)(expr: Term, from: Symbol): Return = Return(expr, from)
    @js("$quoted") def unapply(x: Return): (Term, Symbol)
  object ReturnTypeTest:
    def unapply(x: Tree): Option[Return] = if x.isInstanceOf[Return] then Some(x.asInstanceOf[Return]) else None
  object ReturnMethods:
    extension (self: Return)
      @js("$quoted") def expr: Term
      @js("$quoted") def from: Symbol
  export ReturnMethods.*

  object Repeated:
    @js("$quoted") def apply(elems: List[Term], tpt: TypeTree): Repeated
    def copy(original: Tree)(elems: List[Term], tpt: TypeTree): Repeated = Repeated(elems, tpt)
    @js("$quoted") def unapply(x: Repeated): (List[Term], TypeTree)
  object RepeatedTypeTest:
    def unapply(x: Tree): Option[Repeated] = if x.isInstanceOf[Repeated] then Some(x.asInstanceOf[Repeated]) else None
  object RepeatedMethods:
    extension (self: Repeated)
      @js("$quoted") def elems: List[Term]
      @js("$quoted") def elemtpt: TypeTree
  export RepeatedMethods.*

  object Inlined:
    @js("$quoted") def apply(call: Option[Tree], bindings: List[Definition], expansion: Term): Inlined
    def copy(original: Tree)(call: Option[Tree], bindings: List[Definition], expansion: Term): Inlined = Inlined(call, bindings, expansion)
    @js("$quoted") def unapply(x: Inlined): (Option[Tree], List[Definition], Term)
  object InlinedTypeTest:
    def unapply(x: Tree): Option[Inlined] = if x.isInstanceOf[Inlined] then Some(x.asInstanceOf[Inlined]) else None
  object InlinedMethods:
    extension (self: Inlined)
      @js("$quoted") def call: Option[Tree]
      @js("$quoted") def bindings: List[Definition]
      @js("$quoted") def body: Term
  export InlinedMethods.*

  object SelectOuter:
    @js("$quoted") def apply(qualifier: Term, name: String, levels: Int): SelectOuter
    def copy(original: Tree)(qualifier: Term, name: String, levels: Int): SelectOuter = SelectOuter(qualifier, name, levels)
    @js("$quoted") def unapply(x: SelectOuter): (Term, String, Int)
  object SelectOuterTypeTest:
    def unapply(x: Tree): Option[SelectOuter] = if x.isInstanceOf[SelectOuter] then Some(x.asInstanceOf[SelectOuter]) else None
  object SelectOuterMethods:
    extension (self: SelectOuter)
      @js("$quoted") def qualifier: Term
      @js("$quoted") def name: String
      @js("$quoted") def level: Int
  export SelectOuterMethods.*

  object While:
    @js("$quoted") def apply(cond: Term, body: Term): While
    def copy(original: Tree)(cond: Term, body: Term): While = While(cond, body)
    @js("$quoted") def unapply(x: While): (Term, Term)
  object WhileTypeTest:
    def unapply(x: Tree): Option[While] = if x.isInstanceOf[While] then Some(x.asInstanceOf[While]) else None
  object WhileMethods:
    extension (self: While)
      @js("$quoted") def cond: Term
      @js("$quoted") def body: Term
  export WhileMethods.*

  object TypeTree:
    @js("$quoted") def of[T](using Type[T]): TypeTree
    @js("$quoted") def ref(typeSymbol: Symbol): TypeTree
  object TypeTreeTypeTest:
    def unapply(x: Tree): Option[TypeTree] = if x.isInstanceOf[TypeTree] then Some(x.asInstanceOf[TypeTree]) else None
  object TypeTreeMethods:
    extension (self: TypeTree)
      @js("$quoted") def tpe: TypeRepr
  export TypeTreeMethods.*

  object Inferred:
    @js("$quoted") def apply(tpe: TypeRepr): Inferred
    def unapply(x: Inferred): Boolean = true
  object InferredTypeTest:
    def unapply(x: Tree): Option[Inferred] = if x.isInstanceOf[Inferred] then Some(x.asInstanceOf[Inferred]) else None

  object TypeIdent:
    @js("$quoted") def apply(sym: Symbol): TypeTree
    def copy(original: Tree)(name: String): TypeIdent = original.asInstanceOf[TypeIdent]
    @js("$quoted") def unapply(x: TypeIdent): Some[String]
  object TypeIdentTypeTest:
    def unapply(x: Tree): Option[TypeIdent] = if x.isInstanceOf[TypeIdent] then Some(x.asInstanceOf[TypeIdent]) else None
  object TypeIdentMethods:
    extension (self: TypeIdent)
      @js("$quoted") def name: String
  export TypeIdentMethods.*

  object TypeSelect:
    @js("$quoted") def apply(qualifier: Term, name: String): TypeSelect
    def copy(original: Tree)(qualifier: Term, name: String): TypeSelect = TypeSelect(qualifier, name)
    @js("$quoted") def unapply(x: TypeSelect): (Term, String)
  object TypeSelectTypeTest:
    def unapply(x: Tree): Option[TypeSelect] = if x.isInstanceOf[TypeSelect] then Some(x.asInstanceOf[TypeSelect]) else None
  object TypeSelectMethods:
    extension (self: TypeSelect)
      @js("$quoted") def qualifier: Term
      @js("$quoted") def name: String
  export TypeSelectMethods.*

  object TypeProjection:
    def copy(original: Tree)(qualifier: TypeTree, name: String): TypeProjection = original.asInstanceOf[TypeProjection]
    @js("$quoted") def unapply(x: TypeProjection): (TypeTree, String)
  object TypeProjectionTypeTest:
    def unapply(x: Tree): Option[TypeProjection] = if x.isInstanceOf[TypeProjection] then Some(x.asInstanceOf[TypeProjection]) else None
  object TypeProjectionMethods:
    extension (self: TypeProjection)
      @js("$quoted") def qualifier: TypeTree
      @js("$quoted") def name: String
  export TypeProjectionMethods.*

  object Singleton:
    @js("$quoted") def apply(ref: Term): Singleton
    def copy(original: Tree)(ref: Term): Singleton = Singleton(ref)
    @js("$quoted") def unapply(x: Singleton): Some[Term]
  object SingletonTypeTest:
    def unapply(x: Tree): Option[Singleton] = if x.isInstanceOf[Singleton] then Some(x.asInstanceOf[Singleton]) else None
  object SingletonMethods:
    extension (self: Singleton)
      @js("$quoted") def ref: Term
  export SingletonMethods.*

  object Refined:
    def copy(original: Tree)(tpt: TypeTree, refinements: List[Definition]): Refined = original.asInstanceOf[Refined]
    @js("$quoted") def unapply(x: Refined): (TypeTree, List[Definition])
  object RefinedTypeTest:
    def unapply(x: Tree): Option[Refined] = if x.isInstanceOf[Refined] then Some(x.asInstanceOf[Refined]) else None
  object RefinedMethods:
    extension (self: Refined)
      @js("$quoted") def tpt: TypeTree
      @js("$quoted") def refinements: List[Definition]
  export RefinedMethods.*

  object Applied:
    @js("$quoted") def apply(tpt: TypeTree, args: List[Tree]): Applied
    def copy(original: Tree)(tpt: TypeTree, args: List[Tree]): Applied = Applied(tpt, args)
    @js("$quoted") def unapply(x: Applied): (TypeTree, List[Tree])
  object AppliedTypeTest:
    def unapply(x: Tree): Option[Applied] = if x.isInstanceOf[Applied] then Some(x.asInstanceOf[Applied]) else None
  object AppliedMethods:
    extension (self: Applied)
      @js("$quoted") def tpt: TypeTree
      @js("$quoted") def args: List[Tree]
  export AppliedMethods.*

  object Annotated:
    @js("$quoted") def apply(arg: TypeTree, annotation: Term): Annotated
    def copy(original: Tree)(arg: TypeTree, annotation: Term): Annotated = Annotated(arg, annotation)
    @js("$quoted") def unapply(x: Annotated): (TypeTree, Term)
  object AnnotatedTypeTest:
    def unapply(x: Tree): Option[Annotated] = if x.isInstanceOf[Annotated] then Some(x.asInstanceOf[Annotated]) else None
  object AnnotatedMethods:
    extension (self: Annotated)
      @js("$quoted") def arg: TypeTree
      @js("$quoted") def annotation: Term
  export AnnotatedMethods.*

  object MatchTypeTree:
    @js("$quoted") def unapply(x: MatchTypeTree): (Option[TypeTree], TypeTree, List[TypeCaseDef])
  object MatchTypeTreeTypeTest:
    def unapply(x: Tree): Option[MatchTypeTree] = if x.isInstanceOf[MatchTypeTree] then Some(x.asInstanceOf[MatchTypeTree]) else None
  object MatchTypeTreeMethods:
    extension (self: MatchTypeTree)
      @js("$quoted") def bound: Option[TypeTree]
      @js("$quoted") def selector: TypeTree
      @js("$quoted") def cases: List[TypeCaseDef]
  export MatchTypeTreeMethods.*

  object ByName:
    @js("$quoted") def apply(result: TypeTree): ByName
    def copy(original: Tree)(result: TypeTree): ByName = ByName(result)
    @js("$quoted") def unapply(x: ByName): Some[TypeTree]
  object ByNameTypeTest:
    def unapply(x: Tree): Option[ByName] = if x.isInstanceOf[ByName] then Some(x.asInstanceOf[ByName]) else None
  object ByNameMethods:
    extension (self: ByName)
      @js("$quoted") def result: TypeTree
  export ByNameMethods.*

  object LambdaTypeTree:
    @js("$quoted") def unapply(tree: LambdaTypeTree): (List[TypeDef], Tree)
  object LambdaTypeTreeTypeTest:
    def unapply(x: Tree): Option[LambdaTypeTree] = if x.isInstanceOf[LambdaTypeTree] then Some(x.asInstanceOf[LambdaTypeTree]) else None
  object LambdaTypeTreeMethods:
    extension (self: LambdaTypeTree)
      @js("$quoted") def tparams: List[TypeDef]
      @js("$quoted") def body: Tree
  export LambdaTypeTreeMethods.*

  object TypeBind:
    @js("$quoted") def unapply(x: TypeBind): (String, Tree)
  object TypeBindTypeTest:
    def unapply(x: Tree): Option[TypeBind] = if x.isInstanceOf[TypeBind] then Some(x.asInstanceOf[TypeBind]) else None
  object TypeBindMethods:
    extension (self: TypeBind)
      @js("$quoted") def name: String
      @js("$quoted") def body: Tree
  export TypeBindMethods.*

  object TypeBlock:
    @js("$quoted") def apply(aliases: List[TypeDef], tpt: TypeTree): TypeBlock
    @js("$quoted") def unapply(x: TypeBlock): (List[TypeDef], TypeTree)
  object TypeBlockTypeTest:
    def unapply(x: Tree): Option[TypeBlock] = if x.isInstanceOf[TypeBlock] then Some(x.asInstanceOf[TypeBlock]) else None
  object TypeBlockMethods:
    extension (self: TypeBlock)
      @js("$quoted") def aliases: List[TypeDef]
      @js("$quoted") def tpt: TypeTree
  export TypeBlockMethods.*

  object TypeBoundsTree:
    @js("$quoted") def apply(low: TypeTree, hi: TypeTree): TypeBoundsTree
    @js("$quoted") def copy(original: Tree)(low: TypeTree, hi: TypeTree): TypeBoundsTree
    @js("$quoted") def unapply(x: TypeBoundsTree): (TypeTree, TypeTree)
  object TypeBoundsTreeTypeTest:
    def unapply(x: Tree): Option[TypeBoundsTree] = if x.isInstanceOf[TypeBoundsTree] then Some(x.asInstanceOf[TypeBoundsTree]) else None
  object TypeBoundsTreeMethods:
    extension (self: TypeBoundsTree)
      @js("$quoted") def tpe: TypeBounds
      @js("$quoted") def low: TypeTree
      @js("$quoted") def hi: TypeTree
  export TypeBoundsTreeMethods.*

  object WildcardTypeTree:
    @js("$quoted") def apply(tpe: TypeRepr): WildcardTypeTree
    def unapply(x: WildcardTypeTree): Boolean = true
  object WildcardTypeTreeTypeTest:
    def unapply(x: Tree): Option[WildcardTypeTree] = if x.isInstanceOf[WildcardTypeTree] then Some(x.asInstanceOf[WildcardTypeTree]) else None
  object WildcardTypeTreeMethods:
    extension (self: WildcardTypeTree)
      @js("$quoted") def tpe: TypeRepr
  export WildcardTypeTreeMethods.*

  object CaseDef:
    @js("$quoted") def apply(pattern: Tree, guard: Option[Term], rhs: Term): CaseDef
    def copy(original: Tree)(pattern: Tree, guard: Option[Term], rhs: Term): CaseDef = CaseDef(pattern, guard, rhs)
    @js("$quoted") def unapply(x: CaseDef): (Tree, Option[Term], Term)
  object CaseDefTypeTest:
    def unapply(x: Tree): Option[CaseDef] = if x.isInstanceOf[CaseDef] then Some(x.asInstanceOf[CaseDef]) else None
  object CaseDefMethods:
    extension (self: CaseDef)
      @js("$quoted") def pattern: Tree
      @js("$quoted") def guard: Option[Term]
      @js("$quoted") def rhs: Term
  export CaseDefMethods.*

  object TypeCaseDef:
    @js("$quoted") def unapply(tree: TypeCaseDef): (TypeTree, TypeTree)
  object TypeCaseDefTypeTest:
    def unapply(x: Tree): Option[TypeCaseDef] = if x.isInstanceOf[TypeCaseDef] then Some(x.asInstanceOf[TypeCaseDef]) else None
  object TypeCaseDefMethods:
    extension (self: TypeCaseDef)
      @js("$quoted") def pattern: TypeTree
      @js("$quoted") def rhs: TypeTree
  export TypeCaseDefMethods.*

  object Bind:
    @js("$quoted") def apply(sym: Symbol, pattern: Tree): Bind
    def copy(original: Tree)(name: String, pattern: Tree): Bind = original.asInstanceOf[Bind]
    @js("$quoted") def unapply(pattern: Bind): (String, Tree)
  object BindTypeTest:
    def unapply(x: Tree): Option[Bind] = if x.isInstanceOf[Bind] then Some(x.asInstanceOf[Bind]) else None
  object BindMethods:
    extension (self: Bind)
      @js("$quoted") def name: String
      @js("$quoted") def pattern: Tree
  export BindMethods.*

  object Unapply:
    @js("$quoted") def apply(fun: Term, implicits: List[Term], patterns: List[Tree]): Unapply
    def copy(original: Tree)(fun: Term, implicits: List[Term], patterns: List[Tree]): Unapply = Unapply(fun, implicits, patterns)
    @js("$quoted") def unapply(x: Unapply): (Term, List[Term], List[Tree])
  object UnapplyTypeTest:
    def unapply(x: Tree): Option[Unapply] = if x.isInstanceOf[Unapply] then Some(x.asInstanceOf[Unapply]) else None
  object UnapplyMethods:
    extension (self: Unapply)
      @js("$quoted") def fun: Term
      @js("$quoted") def implicits: List[Term]
      @js("$quoted") def patterns: List[Tree]
  export UnapplyMethods.*

  object Alternatives:
    @js("$quoted") def apply(patterns: List[Tree]): Alternatives
    def copy(original: Tree)(patterns: List[Tree]): Alternatives = Alternatives(patterns)
    @js("$quoted") def unapply(x: Alternatives): Some[List[Tree]]
  object AlternativesTypeTest:
    def unapply(x: Tree): Option[Alternatives] = if x.isInstanceOf[Alternatives] then Some(x.asInstanceOf[Alternatives]) else None
  object AlternativesMethods:
    extension (self: Alternatives)
      @js("$quoted") def patterns: List[Tree]
  export AlternativesMethods.*

  object ParamClauseMethods:
    extension (self: ParamClause)
      def params: List[ValDef] | List[TypeDef] = self.asInstanceOf[List[ValDef]]
  export ParamClauseMethods.*
  object TermParamClause:
    def apply(params: List[ValDef]): TermParamClause = params.asInstanceOf[TermParamClause]
    def unapply(x: TermParamClause): Some[List[ValDef]] = Some(x.asInstanceOf[List[ValDef]])
  object TermParamClauseTypeTest:
    def unapply(x: ParamClause): Option[TermParamClause] = if x.isInstanceOf[TermParamClause] then Some(x.asInstanceOf[TermParamClause]) else None
  object TermParamClauseMethods:
    extension (self: TermParamClause)
      def params: List[ValDef] = self.asInstanceOf[List[ValDef]]
      def isImplicit: Boolean = self.params.headOption.exists(_.symbol.flags.is(Flags.Implicit))
      def isGiven: Boolean = self.params.headOption.exists(_.symbol.flags.is(Flags.Given))
      def isErased: Boolean = false
      def erasedArgs: List[Boolean] = self.params.map(_ => false)
      def hasErasedArgs: Boolean = false
  export TermParamClauseMethods.*
  object TypeParamClause:
    def apply(params: List[TypeDef]): TypeParamClause = params.asInstanceOf[TypeParamClause]
    def unapply(x: TypeParamClause): Some[List[TypeDef]] = Some(x.asInstanceOf[List[TypeDef]])
  object TypeParamClauseTypeTest:
    def unapply(x: ParamClause): Option[TypeParamClause] = if x.isInstanceOf[TypeParamClause] then Some(x.asInstanceOf[TypeParamClause]) else None
  object TypeParamClauseMethods:
    extension (self: TypeParamClause)
      def params: List[TypeDef] = self.asInstanceOf[List[TypeDef]]
  export TypeParamClauseMethods.*

  // ---- types ----

  class TypeRepr
  class NamedType extends TypeRepr
  class TermRef extends NamedType
  class TypeRef extends NamedType
  class ConstantType extends TypeRepr
  class SuperType extends TypeRepr
  class Refinement extends TypeRepr
  class AppliedType extends TypeRepr
  class AnnotatedType extends TypeRepr
  class AndOrType extends TypeRepr
  class AndType extends AndOrType
  class OrType extends AndOrType
  class MatchType extends TypeRepr
  class ByNameType extends TypeRepr
  class ParamRef extends TypeRepr
  class ThisType extends TypeRepr
  class RecursiveThis extends TypeRepr
  class RecursiveType extends TypeRepr
  class LambdaType extends TypeRepr
  class MethodOrPoly extends LambdaType
  class MethodType extends MethodOrPoly
  class PolyType extends MethodOrPoly
  class TypeLambda extends LambdaType
  class MatchCase extends TypeRepr
  class TypeBounds extends TypeRepr
  class NoPrefix extends TypeRepr
  class FlexibleType extends TypeRepr

  object TypeRepr:
    @js("$quoted") def of[T](using Type[T]): TypeRepr
    @js("$quoted") def typeConstructorOf(clazz: Class[?]): TypeRepr
  object TypeReprMethods:
    extension (self: TypeRepr)
      def show(using Printer[TypeRepr]): String = summon[Printer[TypeRepr]].show(self)
      @js("$quoted") def asType: Type[?]
      @js("$quoted") def =:=(that: TypeRepr): Boolean
      @js("$quoted") def <:<(that: TypeRepr): Boolean
      @js("$quoted") def widen: TypeRepr
      @js("$quoted") def widenTermRefByName: TypeRepr
      @js("$quoted") def widenByName: TypeRepr
      @js("$quoted") def dealias: TypeRepr
      def dealiasKeepOpaques: TypeRepr = self.dealias
      @js("$quoted") def simplified: TypeRepr
      @js("$quoted") def classSymbol: Option[Symbol]
      @js("$quoted") def typeSymbol: Symbol
      @js("$quoted") def termSymbol: Symbol
      @js("$quoted") def isSingleton: Boolean
      @js("$quoted") def memberType(member: Symbol): TypeRepr
      @js("$quoted") def baseClasses: List[Symbol]
      @js("$quoted") def baseType(cls: Symbol): TypeRepr
      @js("$quoted") def derivesFrom(cls: Symbol): Boolean
      @js("$quoted") def isFunctionType: Boolean
      def isContextFunctionType: Boolean = false
      def isErasedFunctionType: Boolean = false
      def isDependentFunctionType: Boolean = false
      @js("$quoted") def isTupleN: Boolean
      @js("$quoted") def select(sym: Symbol): TypeRepr
      def appliedTo(targ: TypeRepr): TypeRepr = self.appliedTo(targ :: Nil)
      @js("$quoted") def appliedTo(targs: List[TypeRepr]): TypeRepr
      @js("$quoted") def substituteTypes(from: List[Symbol], to: List[TypeRepr]): TypeRepr
      @js("$quoted") def typeArgs: List[TypeRepr]
  export TypeReprMethods.*

  object ConstantType:
    @js("$quoted") def apply(x: Constant): ConstantType
    @js("$quoted") def unapply(x: ConstantType): Some[Constant]
  object ConstantTypeTypeTest:
    def unapply(x: TypeRepr): Option[ConstantType] = if x.isInstanceOf[ConstantType] then Some(x.asInstanceOf[ConstantType]) else None
  object ConstantTypeMethods:
    extension (self: ConstantType)
      @js("$quoted") def constant: Constant
  export ConstantTypeMethods.*

  object NamedTypeTypeTest:
    def unapply(x: TypeRepr): Option[NamedType] = if x.isInstanceOf[NamedType] then Some(x.asInstanceOf[NamedType]) else None
  object NamedTypeMethods:
    extension (self: NamedType)
      @js("$quoted") def qualifier: TypeRepr
      @js("$quoted") def name: String
  export NamedTypeMethods.*

  object TermRef:
    @js("$quoted") def apply(qual: TypeRepr, name: String): TermRef
    @js("$quoted") def unapply(x: TermRef): (TypeRepr, String)
  object TermRefTypeTest:
    def unapply(x: TypeRepr): Option[TermRef] = if x.isInstanceOf[TermRef] then Some(x.asInstanceOf[TermRef]) else None

  object TypeRef:
    @js("$quoted") def unapply(x: TypeRef): (TypeRepr, String)
  object TypeRefTypeTest:
    def unapply(x: TypeRepr): Option[TypeRef] = if x.isInstanceOf[TypeRef] then Some(x.asInstanceOf[TypeRef]) else None
  object TypeRefMethods:
    extension (self: TypeRef)
      @js("$quoted") def isOpaqueAlias: Boolean
      @js("$quoted") def translucentSuperType: TypeRepr
  export TypeRefMethods.*

  object SuperType:
    @js("$quoted") def unapply(x: SuperType): (TypeRepr, TypeRepr)
  object SuperTypeTypeTest:
    def unapply(x: TypeRepr): Option[SuperType] = if x.isInstanceOf[SuperType] then Some(x.asInstanceOf[SuperType]) else None
  object SuperTypeMethods:
    extension (self: SuperType)
      @js("$quoted") def thistpe: TypeRepr
      @js("$quoted") def supertpe: TypeRepr
  export SuperTypeMethods.*

  object Refinement:
    @js("$quoted") def apply(parent: TypeRepr, name: String, info: TypeRepr): Refinement
    @js("$quoted") def unapply(x: Refinement): (TypeRepr, String, TypeRepr)
  object RefinementTypeTest:
    def unapply(x: TypeRepr): Option[Refinement] = if x.isInstanceOf[Refinement] then Some(x.asInstanceOf[Refinement]) else None
  object RefinementMethods:
    extension (self: Refinement)
      @js("$quoted") def parent: TypeRepr
      @js("$quoted") def name: String
      @js("$quoted") def info: TypeRepr
  export RefinementMethods.*

  object AppliedType:
    @js("$quoted") def apply(tycon: TypeRepr, args: List[TypeRepr]): AppliedType
    @js("$quoted") def unapply(x: AppliedType): (TypeRepr, List[TypeRepr])
  object AppliedTypeTypeTest:
    def unapply(x: TypeRepr): Option[AppliedType] = if x.isInstanceOf[AppliedType] then Some(x.asInstanceOf[AppliedType]) else None
  object AppliedTypeMethods:
    extension (self: AppliedType)
      @js("$quoted") def tycon: TypeRepr
      @js("$quoted") def args: List[TypeRepr]
  export AppliedTypeMethods.*

  object AnnotatedType:
    @js("$quoted") def apply(underlying: TypeRepr, annot: Term): AnnotatedType
    @js("$quoted") def unapply(x: AnnotatedType): (TypeRepr, Term)
  object AnnotatedTypeTypeTest:
    def unapply(x: TypeRepr): Option[AnnotatedType] = if x.isInstanceOf[AnnotatedType] then Some(x.asInstanceOf[AnnotatedType]) else None
  object AnnotatedTypeMethods:
    extension (self: AnnotatedType)
      @js("$quoted") def underlying: TypeRepr
      @js("$quoted") def annotation: Term
  export AnnotatedTypeMethods.*

  object AndOrTypeTypeTest:
    def unapply(x: TypeRepr): Option[AndOrType] = if x.isInstanceOf[AndOrType] then Some(x.asInstanceOf[AndOrType]) else None
  object AndOrTypeMethods:
    extension (self: AndOrType)
      @js("$quoted") def left: TypeRepr
      @js("$quoted") def right: TypeRepr
  export AndOrTypeMethods.*
  object AndType:
    @js("$quoted") def apply(lhs: TypeRepr, rhs: TypeRepr): AndType
    @js("$quoted") def unapply(x: AndType): (TypeRepr, TypeRepr)
  object AndTypeTypeTest:
    def unapply(x: TypeRepr): Option[AndType] = if x.isInstanceOf[AndType] then Some(x.asInstanceOf[AndType]) else None
  object OrType:
    @js("$quoted") def apply(lhs: TypeRepr, rhs: TypeRepr): OrType
    @js("$quoted") def unapply(x: OrType): (TypeRepr, TypeRepr)
  object OrTypeTypeTest:
    def unapply(x: TypeRepr): Option[OrType] = if x.isInstanceOf[OrType] then Some(x.asInstanceOf[OrType]) else None

  object MatchType:
    @js("$quoted") def unapply(x: MatchType): (TypeRepr, TypeRepr, List[TypeRepr])
  object MatchTypeTypeTest:
    def unapply(x: TypeRepr): Option[MatchType] = if x.isInstanceOf[MatchType] then Some(x.asInstanceOf[MatchType]) else None
  object MatchTypeMethods:
    extension (self: MatchType)
      @js("$quoted") def bound: TypeRepr
      @js("$quoted") def scrutinee: TypeRepr
      @js("$quoted") def cases: List[TypeRepr]
  export MatchTypeMethods.*

  object ByNameType:
    @js("$quoted") def apply(underlying: TypeRepr): TypeRepr
    @js("$quoted") def unapply(x: ByNameType): Some[TypeRepr]
  object ByNameTypeTypeTest:
    def unapply(x: TypeRepr): Option[ByNameType] = if x.isInstanceOf[ByNameType] then Some(x.asInstanceOf[ByNameType]) else None
  object ByNameTypeMethods:
    extension (self: ByNameType)
      @js("$quoted") def underlying: TypeRepr
  export ByNameTypeMethods.*

  object ParamRef:
    @js("$quoted") def unapply(x: ParamRef): (TypeRepr, Int)
  object ParamRefTypeTest:
    def unapply(x: TypeRepr): Option[ParamRef] = if x.isInstanceOf[ParamRef] then Some(x.asInstanceOf[ParamRef]) else None
  object ParamRefMethods:
    extension (self: ParamRef)
      @js("$quoted") def binder: TypeRepr
      @js("$quoted") def paramNum: Int
  export ParamRefMethods.*

  object ThisType:
    @js("$quoted") def unapply(x: ThisType): Some[TypeRepr]
  object ThisTypeTypeTest:
    def unapply(x: TypeRepr): Option[ThisType] = if x.isInstanceOf[ThisType] then Some(x.asInstanceOf[ThisType]) else None
  object ThisTypeMethods:
    extension (self: ThisType)
      @js("$quoted") def tref: TypeRepr
  export ThisTypeMethods.*

  object RecursiveThis:
    @js("$quoted") def unapply(x: RecursiveThis): Some[RecursiveType]
  object RecursiveThisTypeTest:
    def unapply(x: TypeRepr): Option[RecursiveThis] = if x.isInstanceOf[RecursiveThis] then Some(x.asInstanceOf[RecursiveThis]) else None
  object RecursiveType:
    @js("$quoted") def unapply(x: RecursiveType): Some[TypeRepr]
  object RecursiveTypeTypeTest:
    def unapply(x: TypeRepr): Option[RecursiveType] = if x.isInstanceOf[RecursiveType] then Some(x.asInstanceOf[RecursiveType]) else None
  object RecursiveTypeMethods:
    extension (self: RecursiveType)
      @js("$quoted") def underlying: TypeRepr
      @js("$quoted") def recThis: RecursiveThis
  export RecursiveTypeMethods.*

  object LambdaTypeTypeTest:
    def unapply(x: TypeRepr): Option[LambdaType] = if x.isInstanceOf[LambdaType] then Some(x.asInstanceOf[LambdaType]) else None
  object LambdaTypeMethods:
    extension (self: LambdaType)
      @js("$quoted") def paramNames: List[String]
      @js("$quoted") def paramTypes: List[TypeRepr]
      @js("$quoted") def resType: TypeRepr
  export LambdaTypeMethods.*
  object MethodOrPolyTypeTest:
    def unapply(x: TypeRepr): Option[MethodOrPoly] = if x.isInstanceOf[MethodOrPoly] then Some(x.asInstanceOf[MethodOrPoly]) else None
  object MethodType:
    @js("$quoted") def apply(paramNames: List[String])(paramInfosExp: MethodType => List[TypeRepr], resultTypeExp: MethodType => TypeRepr): MethodType
    @js("$quoted") def unapply(x: MethodType): (List[String], List[TypeRepr], TypeRepr)
  object MethodTypeTypeTest:
    def unapply(x: TypeRepr): Option[MethodType] = if x.isInstanceOf[MethodType] then Some(x.asInstanceOf[MethodType]) else None
  object MethodTypeMethods:
    extension (self: MethodType)
      @js("$quoted") def isImplicit: Boolean
      @js("$quoted") def isContextual: Boolean
      def isErased: Boolean = false
      def hasErasedParams: Boolean = false
      def erasedParams: List[Boolean] = self.paramTypes.map(_ => false)
      @js("$quoted") def param(idx: Int): TypeRepr
  export MethodTypeMethods.*
  object PolyType:
    @js("$quoted") def unapply(x: PolyType): (List[String], List[TypeBounds], TypeRepr)
  object PolyTypeTypeTest:
    def unapply(x: TypeRepr): Option[PolyType] = if x.isInstanceOf[PolyType] then Some(x.asInstanceOf[PolyType]) else None
  object PolyTypeMethods:
    extension (self: PolyType)
      @js("$quoted") def param(idx: Int): TypeRepr
      @js("$quoted") def paramBounds: List[TypeBounds]
  export PolyTypeMethods.*
  object TypeLambda:
    @js("$quoted") def apply(paramNames: List[String], boundsFn: TypeLambda => List[TypeBounds], bodyFn: TypeLambda => TypeRepr): TypeLambda
    @js("$quoted") def unapply(x: TypeLambda): (List[String], List[TypeBounds], TypeRepr)
  object TypeLambdaTypeTest:
    def unapply(x: TypeRepr): Option[TypeLambda] = if x.isInstanceOf[TypeLambda] then Some(x.asInstanceOf[TypeLambda]) else None
  object TypeLambdaMethods:
    extension (self: TypeLambda)
      @js("$quoted") def param(idx: Int): TypeRepr
      @js("$quoted") def paramBounds: List[TypeBounds]
      @js("$quoted") def paramVariances: List[Flags]
  export TypeLambdaMethods.*

  object MatchCase:
    @js("$quoted") def unapply(x: MatchCase): (TypeRepr, TypeRepr)
  object MatchCaseTypeTest:
    def unapply(x: TypeRepr): Option[MatchCase] = if x.isInstanceOf[MatchCase] then Some(x.asInstanceOf[MatchCase]) else None
  object MatchCaseMethods:
    extension (self: MatchCase)
      @js("$quoted") def pattern: TypeRepr
      @js("$quoted") def rhs: TypeRepr
  export MatchCaseMethods.*

  object TypeBounds:
    @js("$quoted") def apply(low: TypeRepr, hi: TypeRepr): TypeBounds
    def empty: TypeBounds = TypeBounds(TypeRepr.of[Nothing], TypeRepr.of[Any])
    def upper(hi: TypeRepr): TypeBounds = TypeBounds(TypeRepr.of[Nothing], hi)
    def lower(lo: TypeRepr): TypeBounds = TypeBounds(lo, TypeRepr.of[Any])
    @js("$quoted") def unapply(x: TypeBounds): (TypeRepr, TypeRepr)
  object TypeBoundsTypeTest:
    def unapply(x: TypeRepr): Option[TypeBounds] = if x.isInstanceOf[TypeBounds] then Some(x.asInstanceOf[TypeBounds]) else None
  object TypeBoundsMethods:
    extension (self: TypeBounds)
      @js("$quoted") def low: TypeRepr
      @js("$quoted") def hi: TypeRepr
  export TypeBoundsMethods.*

  object NoPrefix:
    def unapply(x: NoPrefix): Boolean = true
  object NoPrefixTypeTest:
    def unapply(x: TypeRepr): Option[NoPrefix] = if x.isInstanceOf[NoPrefix] then Some(x.asInstanceOf[NoPrefix]) else None
  object FlexibleType:
    @js("$quoted") def unapply(x: FlexibleType): Some[TypeRepr]
  object FlexibleTypeTypeTest:
    def unapply(x: TypeRepr): Option[FlexibleType] = if x.isInstanceOf[FlexibleType] then Some(x.asInstanceOf[FlexibleType]) else None
  object FlexibleTypeMethods:
    extension (self: FlexibleType)
      @js("$quoted") def underlying: TypeRepr
      @js("$quoted") def lo: TypeRepr
      @js("$quoted") def hi: TypeRepr
  export FlexibleTypeMethods.*

  // ---- constants ----

  // A constant is its value and its kind, the tag of dotty's `Constants.Constant` (`UnitTag` 2 to
  // `ClazzTag` 13), which the kinds' type tests and extractors compare (`QuotesImpl.IntConstantTypeTest`:
  // `x.tag == IntTag`), never the value's class: on JavaScript a `Char` is no other than a one-character
  // `String` but by its tag.
  final class Constant(val value: Any, val tag: Int)
  // scalac's `val Constant: ConstantModule`, a module with no members: a constant is made by its kind's.
  object Constant
  object ConstantMethods:
    extension (self: Constant)
      def value: Any = self.value
      def show(using Printer[Constant]): String = summon[Printer[Constant]].show(self)
  export ConstantMethods.*
  object BooleanConstant:
    def apply(x: Boolean): BooleanConstant = new Constant(x, 3)
    def unapply(c: Constant): Option[Boolean] = if c.tag == 3 then Some(c.value.asInstanceOf[Boolean]) else None
  object ByteConstant:
    def apply(x: Byte): ByteConstant = new Constant(x, 4)
    def unapply(c: Constant): Option[Byte] = if c.tag == 4 then Some(c.value.asInstanceOf[Byte]) else None
  object ShortConstant:
    def apply(x: Short): ShortConstant = new Constant(x, 5)
    def unapply(c: Constant): Option[Short] = if c.tag == 5 then Some(c.value.asInstanceOf[Short]) else None
  object IntConstant:
    def apply(x: Int): IntConstant = new Constant(x, 7)
    def unapply(c: Constant): Option[Int] = if c.tag == 7 then Some(c.value.asInstanceOf[Int]) else None
  object LongConstant:
    def apply(x: Long): LongConstant = new Constant(x, 8)
    def unapply(c: Constant): Option[Long] = if c.tag == 8 then Some(c.value.asInstanceOf[Long]) else None
  object FloatConstant:
    def apply(x: Float): FloatConstant = new Constant(x, 9)
    def unapply(c: Constant): Option[Float] = if c.tag == 9 then Some(c.value.asInstanceOf[Float]) else None
  object DoubleConstant:
    def apply(x: Double): DoubleConstant = new Constant(x, 10)
    def unapply(c: Constant): Option[Double] = if c.tag == 10 then Some(c.value.asInstanceOf[Double]) else None
  object CharConstant:
    def apply(x: Char): CharConstant = new Constant(x, 6)
    def unapply(c: Constant): Option[Char] = if c.tag == 6 then Some(c.value.asInstanceOf[Char]) else None
  object StringConstant:
    def apply(x: String): StringConstant = new Constant(x, 11)
    def unapply(c: Constant): Option[String] = if c.tag == 11 then Some(c.value.asInstanceOf[String]) else None
  object UnitConstant:
    def apply(): UnitConstant = new Constant((), 2)
    def unapply(c: Constant): Boolean = c.tag == 2
  object NullConstant:
    def apply(): NullConstant = new Constant(null, 12)
    def unapply(c: Constant): Boolean = c.tag == 12
  object ClassOfConstant:
    @js("$quoted") def apply(x: TypeRepr): ClassOfConstant
    def unapply(c: Constant): Option[TypeRepr] = if c.tag == 13 then Some(c.value.asInstanceOf[TypeRepr]) else None

  // The kinds of constant: scalac's abstract types under `Constant` (`Quotes.reflectModule`), each told apart by
  // its tag by the `TypeTest` a pattern over it finds in this object (`QuotesImpl.IntConstantTypeTest`).
  opaque type BooleanConstant <: Constant = Constant
  given BooleanConstantTypeTest: scala.reflect.TypeTest[Constant, BooleanConstant] with
    def unapply(x: Constant): Option[x.type & BooleanConstant] = if x.tag == 3 then Some(x) else None
  opaque type ByteConstant <: Constant = Constant
  given ByteConstantTypeTest: scala.reflect.TypeTest[Constant, ByteConstant] with
    def unapply(x: Constant): Option[x.type & ByteConstant] = if x.tag == 4 then Some(x) else None
  opaque type ShortConstant <: Constant = Constant
  given ShortConstantTypeTest: scala.reflect.TypeTest[Constant, ShortConstant] with
    def unapply(x: Constant): Option[x.type & ShortConstant] = if x.tag == 5 then Some(x) else None
  opaque type IntConstant <: Constant = Constant
  given IntConstantTypeTest: scala.reflect.TypeTest[Constant, IntConstant] with
    def unapply(x: Constant): Option[x.type & IntConstant] = if x.tag == 7 then Some(x) else None
  opaque type LongConstant <: Constant = Constant
  given LongConstantTypeTest: scala.reflect.TypeTest[Constant, LongConstant] with
    def unapply(x: Constant): Option[x.type & LongConstant] = if x.tag == 8 then Some(x) else None
  opaque type FloatConstant <: Constant = Constant
  given FloatConstantTypeTest: scala.reflect.TypeTest[Constant, FloatConstant] with
    def unapply(x: Constant): Option[x.type & FloatConstant] = if x.tag == 9 then Some(x) else None
  opaque type DoubleConstant <: Constant = Constant
  given DoubleConstantTypeTest: scala.reflect.TypeTest[Constant, DoubleConstant] with
    def unapply(x: Constant): Option[x.type & DoubleConstant] = if x.tag == 10 then Some(x) else None
  opaque type CharConstant <: Constant = Constant
  given CharConstantTypeTest: scala.reflect.TypeTest[Constant, CharConstant] with
    def unapply(x: Constant): Option[x.type & CharConstant] = if x.tag == 6 then Some(x) else None
  opaque type StringConstant <: Constant = Constant
  given StringConstantTypeTest: scala.reflect.TypeTest[Constant, StringConstant] with
    def unapply(x: Constant): Option[x.type & StringConstant] = if x.tag == 11 then Some(x) else None
  opaque type UnitConstant <: Constant = Constant
  given UnitConstantTypeTest: scala.reflect.TypeTest[Constant, UnitConstant] with
    def unapply(x: Constant): Option[x.type & UnitConstant] = if x.tag == 2 then Some(x) else None
  opaque type NullConstant <: Constant = Constant
  given NullConstantTypeTest: scala.reflect.TypeTest[Constant, NullConstant] with
    def unapply(x: Constant): Option[x.type & NullConstant] = if x.tag == 12 then Some(x) else None
  opaque type ClassOfConstant <: Constant = Constant
  given ClassOfConstantTypeTest: scala.reflect.TypeTest[Constant, ClassOfConstant] with
    def unapply(x: Constant): Option[x.type & ClassOfConstant] = if x.tag == 13 then Some(x) else None

  // ---- implicits ----

  class ImplicitSearchResult
  final class ImplicitSearchSuccess(val tree: Term) extends ImplicitSearchResult
  class ImplicitSearchFailure(val explanation: String) extends ImplicitSearchResult
  final class DivergingImplicit(explanation: String) extends ImplicitSearchFailure(explanation)
  final class NoMatchingImplicits(explanation: String) extends ImplicitSearchFailure(explanation)
  final class AmbiguousImplicits(explanation: String) extends ImplicitSearchFailure(explanation)
  object Implicits:
    @js("$quoted") def search(tpe: TypeRepr): ImplicitSearchResult
    def searchIgnoring(tpe: TypeRepr)(ignored: Symbol*): ImplicitSearchResult = search(tpe)
  object ImplicitSearchSuccessTypeTest:
    def unapply(x: ImplicitSearchResult): Option[ImplicitSearchSuccess] = if x.isInstanceOf[ImplicitSearchSuccess] then Some(x.asInstanceOf[ImplicitSearchSuccess]) else None
  object ImplicitSearchFailureTypeTest:
    def unapply(x: ImplicitSearchResult): Option[ImplicitSearchFailure] = if x.isInstanceOf[ImplicitSearchFailure] then Some(x.asInstanceOf[ImplicitSearchFailure]) else None
  object ImplicitSearchSuccessMethods:
    def tree(self: ImplicitSearchSuccess): Term = self.tree
  object ImplicitSearchFailureMethods:
    def explanation(self: ImplicitSearchFailure): String = self.explanation

  // ---- symbols ----

  final class Symbol
  object Symbol:
    @js("$quoted") def spliceOwner: Symbol
    @js("$quoted") def requiredPackage(path: String): Symbol
    @js("$quoted") def requiredClass(path: String): Symbol
    @js("$quoted") def requiredModule(path: String): Symbol
    @js("$quoted") def requiredMethod(path: String): Symbol
    @js("$quoted") def classSymbol(fullName: String): Symbol
    @js("$quoted") def newClass(parent: Symbol, name: String, parents: List[TypeRepr], decls: Symbol => List[Symbol], selfType: Option[TypeRepr]): Symbol
    @js("$quoted") def newModule(owner: Symbol, name: String, modFlags: Flags, clsFlags: Flags, parents: List[TypeRepr], decls: Symbol => List[Symbol], privateWithin: Symbol): Symbol
    @js("$quoted") def newMethod(parent: Symbol, name: String, tpe: TypeRepr): Symbol
    @js("$quoted") def newMethod(parent: Symbol, name: String, tpe: TypeRepr, flags: Flags, privateWithin: Symbol): Symbol
    @js("$quoted") def newVal(parent: Symbol, name: String, tpe: TypeRepr, flags: Flags, privateWithin: Symbol): Symbol
    @js("$quoted") def newBind(parent: Symbol, name: String, flags: Flags, tpe: TypeRepr): Symbol
    @js("$quoted") def noSymbol: Symbol
    @js("$quoted") def freshName(prefix: String): String
  object SymbolMethods:
    extension (self: Symbol)
      @js("$quoted") def owner: Symbol
      @js("$quoted") def maybeOwner: Symbol
      @js("$quoted") def flags: Flags
      @js("$quoted") def privateWithin: Option[TypeRepr]
      @js("$quoted") def protectedWithin: Option[TypeRepr]
      @js("$quoted") def name: String
      @js("$quoted") def fullName: String
      @js("$quoted") def pos: Option[Position]
      def docstring: Option[String] = None
      @js("$quoted") def tree: Tree
      @js("$quoted") def hasAnnotation(annotSym: Symbol): Boolean
      @js("$quoted") def getAnnotation(annotSym: Symbol): Option[Term]
      @js("$quoted") def annotations: List[Term]
      @js("$quoted") def isDefinedInCurrentRun: Boolean
      @js("$quoted") def isLocalDummy: Boolean
      @js("$quoted") def isRefinementClass: Boolean
      @js("$quoted") def isAliasType: Boolean
      @js("$quoted") def isAnonymousClass: Boolean
      @js("$quoted") def isAnonymousFunction: Boolean
      @js("$quoted") def isAbstractType: Boolean
      @js("$quoted") def isClassConstructor: Boolean
      @js("$quoted") def isSuperAccessor: Boolean
      @js("$quoted") def isType: Boolean
      @js("$quoted") def isTerm: Boolean
      @js("$quoted") def isPackageDef: Boolean
      @js("$quoted") def isClassDef: Boolean
      @js("$quoted") def isTypeDef: Boolean
      @js("$quoted") def isValDef: Boolean
      @js("$quoted") def isDefDef: Boolean
      @js("$quoted") def isBind: Boolean
      @js("$quoted") def isNoSymbol: Boolean
      def exists: Boolean = !self.isNoSymbol
      @js("$quoted") def declaredField(name: String): Symbol
      @js("$quoted") def declaredFields: List[Symbol]
      @js("$quoted") def fieldMember(name: String): Symbol
      @js("$quoted") def fieldMembers: List[Symbol]
      @js("$quoted") def declaredMethod(name: String): List[Symbol]
      @js("$quoted") def declaredMethods: List[Symbol]
      @js("$quoted") def methodMember(name: String): List[Symbol]
      @js("$quoted") def methodMembers: List[Symbol]
      def memberMethod(name: String): List[Symbol] = self.methodMember(name)
      def memberMethods: List[Symbol] = self.methodMembers
      def memberField(name: String): Symbol = self.fieldMember(name)
      def memberFields: List[Symbol] = self.fieldMembers
      def memberType(name: String): Symbol = self.typeMember(name)
      def memberTypes: List[Symbol] = self.typeMembers
      @js("$quoted") def declaredType(name: String): List[Symbol]
      @js("$quoted") def declaredTypes: List[Symbol]
      @js("$quoted") def typeMember(name: String): Symbol
      @js("$quoted") def typeMembers: List[Symbol]
      @js("$quoted") def declarations: List[Symbol]
      @js("$quoted") def paramSymss: List[List[Symbol]]
      @js("$quoted") def allOverriddenSymbols: Iterator[Symbol]
      @js("$quoted") def overridingSymbol(ofclazz: Symbol): Symbol
      @js("$quoted") def primaryConstructor: Symbol
      @js("$quoted") def caseFields: List[Symbol]
      @js("$quoted") def isTypeParam: Boolean
      @js("$quoted") def paramVariance: Flags
      @js("$quoted") def signature: Signature
      @js("$quoted") def moduleClass: Symbol
      @js("$quoted") def companionClass: Symbol
      @js("$quoted") def companionModule: Symbol
      @js("$quoted") def children: List[Symbol]
      @js("$quoted") def typeRef: TypeRef
      @js("$quoted") def termRef: TermRef
      @js("$quoted") def info: TypeRepr
      @js("$quoted") def asQuotes: Quotes
  export SymbolMethods.*

  final class Signature(val paramSigs: List[String | Int], val resultSig: String)
  object Signature:
    def unapply(sig: Signature): (List[String | Int], String) = (sig.paramSigs, sig.resultSig)
  object SignatureMethods:
    def paramSigs(self: Signature): List[String | Int] = self.paramSigs
    def resultSig(self: Signature): String = self.resultSig

  object defn:
    @js("$quoted") def RootPackage: Symbol
    @js("$quoted") def RootClass: Symbol
    @js("$quoted") def EmptyPackageClass: Symbol
    @js("$quoted") def ScalaPackage: Symbol
    @js("$quoted") def ScalaPackageClass: Symbol
    def AnyClass: Symbol = Symbol.classSymbol("scala.Any")
    def MatchableClass: Symbol = Symbol.classSymbol("scala.Matchable")
    def AnyValClass: Symbol = Symbol.classSymbol("scala.AnyVal")
    def ObjectClass: Symbol = Symbol.classSymbol("java.lang.Object")
    def AnyRefClass: Symbol = Symbol.classSymbol("scala.AnyRef")
    def NullClass: Symbol = Symbol.classSymbol("scala.Null")
    def NothingClass: Symbol = Symbol.classSymbol("scala.Nothing")
    def UnitClass: Symbol = Symbol.classSymbol("scala.Unit")
    def ByteClass: Symbol = Symbol.classSymbol("scala.Byte")
    def ShortClass: Symbol = Symbol.classSymbol("scala.Short")
    def CharClass: Symbol = Symbol.classSymbol("scala.Char")
    def IntClass: Symbol = Symbol.classSymbol("scala.Int")
    def LongClass: Symbol = Symbol.classSymbol("scala.Long")
    def FloatClass: Symbol = Symbol.classSymbol("scala.Float")
    def DoubleClass: Symbol = Symbol.classSymbol("scala.Double")
    def BooleanClass: Symbol = Symbol.classSymbol("scala.Boolean")
    def StringClass: Symbol = Symbol.classSymbol("java.lang.String")
    def ClassClass: Symbol = Symbol.classSymbol("java.lang.Class")
    def ArrayClass: Symbol = Symbol.classSymbol("scala.Array")
    def PredefModule: Symbol = Symbol.requiredModule("scala.Predef")
    def Predef_classOf: Symbol = PredefModule.declaredMethod("classOf").head
    def JavaLangPackage: Symbol = Symbol.requiredPackage("java.lang")
    def ArrayModule: Symbol = Symbol.requiredModule("scala.Array")
    def Array_apply: Symbol = ArrayModule.declaredMethod("apply").head
    def Array_clone: Symbol = ArrayClass.methodMember("clone").head
    def Array_length: Symbol = ArrayClass.methodMember("length").head
    def Array_update: Symbol = ArrayClass.methodMember("update").head
    @js("$quoted") def RepeatedParamClass: Symbol
    @js("$quoted") def RepeatedAnnot: Symbol
    def OptionClass: Symbol = Symbol.classSymbol("scala.Option")
    def NoneModule: Symbol = Symbol.requiredModule("scala.None")
    def SomeModule: Symbol = Symbol.requiredModule("scala.Some")
    def ProductClass: Symbol = Symbol.classSymbol("scala.Product")
    @js("$quoted") def FunctionClass(arity: Int): Symbol
    def FunctionClass(arity: Int, isImplicit: Boolean = false, isErased: Boolean = false): Symbol = FunctionClass(arity)
    def FunctionClass(arity: Int, isContextual: Boolean): Symbol = FunctionClass(arity)
    @js("$quoted") def PolyFunctionClass: Symbol
    @js("$quoted") def TupleClass(arity: Int): Symbol
    @js("$quoted") def isTupleClass(sym: Symbol): Boolean
    @js("$quoted") def EmptyTupleClass: Symbol
    @js("$quoted") def NonEmptyTupleClass: Symbol
    def ScalaPrimitiveValueClasses: List[Symbol] = UnitClass :: BooleanClass :: ScalaNumericValueClasses
    def ScalaNumericValueClasses: List[Symbol] = ByteClass :: ShortClass :: IntClass :: LongClass :: FloatClass :: DoubleClass :: CharClass :: Nil

  trait CompilationInfoModule:
    def isWhileTyping: Boolean
    def XmacroSettings: List[String]
  object CompilationInfo extends CompilationInfoModule:
    def isWhileTyping: Boolean = false
    def XmacroSettings: List[String] = Nil

  // ---- flags ----

  final class Flags(val bits: Long)
  object FlagsMethods:
    extension (self: Flags)
      def is(that: Flags): Boolean = (self.bits & that.bits) == that.bits
      def |(that: Flags): Flags = new Flags(self.bits | that.bits)
      def &(that: Flags): Flags = new Flags(self.bits & that.bits)
      def show: String = Flags.names.filter((_, f) => self.is(f)).map((n, _) => n).mkString(" ")
  export FlagsMethods.*
  object Flags:
    private def bit(n: Int): Flags = new Flags(1L << n)
    val Abstract: Flags = bit(0)
    val Artifact: Flags = bit(1)
    val Case: Flags = bit(2)
    val CaseAccessor: Flags = bit(3)
    val Contravariant: Flags = bit(4)
    val Covariant: Flags = bit(5)
    val Deferred: Flags = bit(6)
    val EmptyFlags: Flags = new Flags(0L)
    val Enum: Flags = bit(7)
    val Erased: Flags = bit(8)
    val Exported: Flags = bit(9)
    val ExtensionMethod: Flags = bit(10)
    val FieldAccessor: Flags = bit(11)
    val Final: Flags = bit(12)
    val Given: Flags = bit(13)
    val HasDefault: Flags = bit(14)
    val Implicit: Flags = bit(15)
    val Infix: Flags = bit(16)
    val Inline: Flags = bit(17)
    val Invariant: Flags = bit(18)
    val JavaDefined: Flags = bit(19)
    val JavaStatic: Flags = bit(20)
    val JavaAnnotation: Flags = bit(21)
    val Lazy: Flags = bit(22)
    val Local: Flags = bit(23)
    val Macro: Flags = bit(24)
    val Method: Flags = bit(25)
    val Module: Flags = bit(26)
    val Mutable: Flags = bit(27)
    val NoInits: Flags = bit(28)
    val Opaque: Flags = bit(29)
    val Open: Flags = bit(30)
    val Override: Flags = bit(31)
    val Package: Flags = bit(32)
    val Param: Flags = bit(33)
    val ParamAccessor: Flags = bit(34)
    val Private: Flags = bit(35)
    val PrivateLocal: Flags = bit(36)
    val Protected: Flags = bit(37)
    val Scala2x: Flags = bit(38)
    val Sealed: Flags = bit(39)
    val StableRealizable: Flags = bit(40)
    val Static: Flags = bit(41)
    val Synthetic: Flags = bit(42)
    val Trait: Flags = bit(43)
    val Transparent: Flags = bit(44)
    val AbsOverride: Flags = bit(45)
    val Tracked: Flags = bit(46)
    val names: List[(String, Flags)] = List(
      ("abstract", Abstract), ("artifact", Artifact), ("case", Case), ("caseaccessor", CaseAccessor),
      ("contravariant", Contravariant), ("covariant", Covariant), ("deferred", Deferred), ("enum", Enum),
      ("erased", Erased), ("exported", Exported), ("extension", ExtensionMethod), ("accessor", FieldAccessor),
      ("final", Final), ("given", Given), ("hasDefault", HasDefault), ("implicit", Implicit), ("infix", Infix),
      ("inline", Inline), ("javaDefined", JavaDefined), ("static", JavaStatic), ("lazy", Lazy), ("local", Local),
      ("macro", Macro), ("method", Method), ("object", Module), ("mutable", Mutable), ("opaque", Opaque),
      ("open", Open), ("override", Override), ("package", Package), ("param", Param), ("paramAccessor", ParamAccessor),
      ("private", Private), ("protected", Protected), ("sealed", Sealed), ("stable", StableRealizable),
      ("synthetic", Synthetic), ("trait", Trait), ("transparent", Transparent))

  // ---- positions ----

  final class Position
  object Position:
    @js("$quoted") def ofMacroExpansion: Position
    @js("$quoted") def apply(sourceFile: SourceFile, start: Int, end: Int): Position
  object PositionMethods:
    extension (self: Position)
      @js("$quoted") def start: Int
      @js("$quoted") def end: Int
      @js("$quoted") def sourceFile: SourceFile
      @js("$quoted") def startLine: Int
      @js("$quoted") def endLine: Int
      @js("$quoted") def startColumn: Int
      @js("$quoted") def endColumn: Int
      @js("$quoted") def sourceCode: Option[String]
  export PositionMethods.*
  final class SourceFile
  object SourceFile:
    @js("$quoted") def current: SourceFile
  object SourceFileMethods:
    extension (self: SourceFile)
      @js("$quoted") def getJPath: Option[java.nio.file.Path]
      @js("$quoted") def jpath: java.nio.file.Path
      @js("$quoted") def name: String
      @js("$quoted") def path: String
      @js("$quoted") def content: Option[String]
  export SourceFileMethods.*

  // ---- reporting ----

  object report:
    @js("$quoted") def error(msg: String): Unit
    @js("$quoted") def error(msg: String, expr: Expr[Any]): Unit
    @js("$quoted") def error(msg: String, pos: Position): Unit
    @js("$quoted") def errorAndAbort(msg: String): Nothing
    @js("$quoted") def errorAndAbort(msg: String, expr: Expr[Any]): Nothing
    @js("$quoted") def errorAndAbort(msg: String, pos: Position): Nothing
    def throwError(msg: String): Nothing = errorAndAbort(msg)
    def throwError(msg: String, expr: Expr[Any]): Nothing = errorAndAbort(msg, expr)
    def throwError(msg: String, pos: Position): Nothing = errorAndAbort(msg, pos)
    @js("$quoted") def warning(msg: String): Unit
    @js("$quoted") def warning(msg: String, expr: Expr[Any]): Unit
    @js("$quoted") def warning(msg: String, pos: Position): Unit
    @js("$quoted") def info(msg: String): Unit
    @js("$quoted") def info(msg: String, expr: Expr[Any]): Unit
    @js("$quoted") def info(msg: String, pos: Position): Unit

  // ---- printers ----

  trait Printer[T]:
    def show(x: T): String
  object Printer:
    object TreeCode extends Printer[Tree]:
      @js("$quoted") def show(tree: Tree): String
    object TreeShortCode extends Printer[Tree]:
      @js("$quoted") def show(tree: Tree): String
    object TreeAnsiCode extends Printer[Tree]:
      def show(tree: Tree): String = TreeCode.show(tree)
    object TreeStructure extends Printer[Tree]:
      @js("$quoted") def show(tree: Tree): String
    object TypeReprCode extends Printer[TypeRepr]:
      @js("$quoted") def show(tpe: TypeRepr): String
    object TypeReprShortCode extends Printer[TypeRepr]:
      @js("$quoted") def show(tpe: TypeRepr): String
    object TypeReprAnsiCode extends Printer[TypeRepr]:
      def show(tpe: TypeRepr): String = TypeReprCode.show(tpe)
    object TypeReprStructure extends Printer[TypeRepr]:
      @js("$quoted") def show(tpe: TypeRepr): String
    object ConstantCode extends Printer[Constant]:
      @js("$quoted") def show(const: Constant): String
    object ConstantStructure extends Printer[Constant]:
      def show(const: Constant): String = "Constant(" + ConstantCode.show(const) + ")"
  given TreePrinter: Printer[Tree] = Printer.TreeCode
  given TypeReprPrinter: Printer[TypeRepr] = Printer.TypeReprCode
  given ConstantPrinter: Printer[Constant] = Printer.ConstantCode

  // ---- traversal ----

  trait TreeAccumulator[X]:
    def foldTree(x: X, tree: Tree)(owner: Symbol): X
    def foldTrees(x: X, trees: Iterable[Tree])(owner: Symbol): X = trees.foldLeft(x)((acc, y) => foldTree(acc, y)(owner))
    def foldOverTree(x: X, tree: Tree)(owner: Symbol): X =
      tree match
        case Ident(_) => x
        case Select(qualifier, _) => foldTree(x, qualifier)(owner)
        case This(qual) => x
        case Super(qual, _) => foldTree(x, qual)(owner)
        case Apply(fun, args) => foldTrees(foldTree(x, fun)(owner), args)(owner)
        case TypeApply(fun, args) => foldTrees(foldTree(x, fun)(owner), args)(owner)
        case Literal(const) => x
        case New(tpt) => foldTree(x, tpt)(owner)
        case Typed(expr, tpt) => foldTree(foldTree(x, expr)(owner), tpt)(owner)
        case TypedOrTest(expr, tpt) => foldTree(foldTree(x, expr)(owner), tpt)(owner)
        case NamedArg(_, arg) => foldTree(x, arg)(owner)
        case Assign(lhs, rhs) => foldTree(foldTree(x, lhs)(owner), rhs)(owner)
        case Block(stats, expr) => foldTree(foldTrees(x, stats)(owner), expr)(owner)
        case If(cond, thenp, elsep) => foldTree(foldTree(foldTree(x, cond)(owner), thenp)(owner), elsep)(owner)
        case While(cond, body) => foldTree(foldTree(x, cond)(owner), body)(owner)
        case Closure(meth, tpt) => foldTree(x, meth)(owner)
        case Match(selector, cases) => foldTrees(foldTree(x, selector)(owner), cases)(owner)
        case Return(expr, _) => foldTree(x, expr)(owner)
        case Try(block, handler, finalizer) => foldTrees(foldTrees(foldTree(x, block)(owner), handler)(owner), finalizer.toList)(owner)
        case Repeated(elems, elemtpt) => foldTrees(foldTree(x, elemtpt)(owner), elems)(owner)
        case Inlined(call, bindings, expansion) => foldTree(foldTrees(x, bindings)(owner), expansion)(owner)
        case vdef @ ValDef(_, tpt, rhs) =>
          val owner = vdef.symbol
          foldTrees(foldTree(x, tpt)(owner), rhs.toList)(owner)
        case ddef @ DefDef(_, paramss, tpt, rhs) =>
          val owner = ddef.symbol
          foldTrees(foldTree(paramss.foldLeft(x)((acc, y) => foldTrees(acc, y.params)(owner)), tpt)(owner), rhs.toList)(owner)
        case tdef @ TypeDef(_, rhs) =>
          val owner = tdef.symbol
          foldTree(x, rhs)(owner)
        case cdef @ ClassDef(_, constr, parents, self, body) =>
          val owner = cdef.symbol
          foldTrees(foldTrees(foldTrees(foldTree(x, constr)(owner), parents)(owner), self.toList)(owner), body)(owner)
        case Import(expr, _) => foldTree(x, expr)(owner)
        case Export(expr, _) => foldTree(x, expr)(owner)
        case clause @ PackageClause(pid, stats) => foldTrees(foldTree(x, pid)(owner), stats)(clause.symbol)
        case Inferred() => x
        case TypeIdent(_) => x
        case TypeSelect(qualifier, _) => foldTree(x, qualifier)(owner)
        case TypeProjection(qualifier, _) => foldTree(x, qualifier)(owner)
        case Singleton(ref) => foldTree(x, ref)(owner)
        case Refined(tpt, refinements) => foldTrees(foldTree(x, tpt)(owner), refinements)(owner)
        case Applied(tpt, args) => foldTrees(foldTree(x, tpt)(owner), args)(owner)
        case ByName(result) => foldTree(x, result)(owner)
        case Annotated(arg, annot) => foldTree(foldTree(x, arg)(owner), annot)(owner)
        case LambdaTypeTree(typedefs, arg) => foldTree(foldTrees(x, typedefs)(owner), arg)(owner)
        case TypeBind(_, tbt) => foldTree(x, tbt)(owner)
        case TypeBlock(typedefs, tpt) => foldTree(foldTrees(x, typedefs)(owner), tpt)(owner)
        case MatchTypeTree(boundopt, selector, cases) =>
          foldTrees(foldTree(boundopt.fold(x)(y => foldTree(x, y)(owner)), selector)(owner), cases)(owner)
        case WildcardTypeTree() => x
        case TypeBoundsTree(lo, hi) => foldTree(foldTree(x, lo)(owner), hi)(owner)
        case CaseDef(pat, guard, body) => foldTree(foldTrees(foldTree(x, pat)(owner), guard.toList)(owner), body)(owner)
        case TypeCaseDef(pat, body) => foldTree(foldTree(x, pat)(owner), body)(owner)
        case Bind(_, body) => foldTree(x, body)(owner)
        case Unapply(fun, implicits, patterns) => foldTrees(foldTrees(foldTree(x, fun)(owner), implicits)(owner), patterns)(owner)
        case Alternatives(patterns) => foldTrees(x, patterns)(owner)
        case _ => x

  trait TreeTraverser extends TreeAccumulator[Unit]:
    def traverseTree(tree: Tree)(owner: Symbol): Unit = traverseTreeChildren(tree)(owner)
    def foldTree(x: Unit, tree: Tree)(owner: Symbol): Unit = traverseTree(tree)(owner)
    protected def traverseTreeChildren(tree: Tree)(owner: Symbol): Unit = foldOverTree((), tree)(owner)

  trait TreeMap:
    def transformTree(tree: Tree)(owner: Symbol): Tree =
      tree match
        case tree: PackageClause => PackageClause.copy(tree)(transformTerm(tree.pid)(owner).asInstanceOf[Ref], transformTrees(tree.stats)(tree.symbol))
        case tree: Statement => transformStatement(tree)(owner)
        case tree: TypeTree => transformTypeTree(tree)(owner)
        case tree: TypeBoundsTree => TypeBoundsTree.copy(tree)(transformTypeTree(tree.low)(owner), transformTypeTree(tree.hi)(owner))
        case tree: WildcardTypeTree => tree
        case tree: CaseDef => transformCaseDef(tree)(owner)
        case tree: TypeCaseDef => transformTypeCaseDef(tree)(owner)
        case pattern: Bind => Bind.copy(pattern)(pattern.name, transformTree(pattern.pattern)(owner))
        case pattern: Unapply => Unapply.copy(pattern)(transformTerm(pattern.fun)(owner), transformSubTrees(pattern.implicits)(owner), transformTrees(pattern.patterns)(owner))
        case pattern: Alternatives => Alternatives.copy(pattern)(transformTrees(pattern.patterns)(owner))
        case TypedOrTest(inner, tpt) => TypedOrTest.copy(tree)(transformTree(inner)(owner), transformTypeTree(tpt)(owner))
    def transformStatement(tree: Statement)(owner: Symbol): Statement =
      tree match
        case tree: Term => transformTerm(tree)(owner)
        case tree: ValDef =>
          val owner = tree.symbol
          val tpt1 = transformTypeTree(tree.tpt)(owner)
          val rhs1 = tree.rhs.map(x => transformTerm(x)(owner))
          ValDef.copy(tree)(tree.name, tpt1, rhs1)
        case tree: DefDef =>
          val owner = tree.symbol
          val newParamClauses = tree.paramss.mapConserve {
            case TypeParamClause(params) => TypeParamClause(transformSubTrees(params)(owner))
            case TermParamClause(params) => TermParamClause(transformSubTrees(params)(owner))
          }
          DefDef.copy(tree)(tree.name, newParamClauses, transformTypeTree(tree.returnTpt)(owner), tree.rhs.map(x => transformTerm(x)(owner)))
        case tree: TypeDef =>
          val owner = tree.symbol
          TypeDef.copy(tree)(tree.name, transformTree(tree.rhs)(owner))
        case tree: ClassDef =>
          val constructor = transformSubTree(tree.constructor)(tree.symbol)
          val parents = tree.parents.map(x => transformTree(x)(tree.symbol))
          val self = tree.self.map(x => transformSubTree(x)(tree.symbol))
          val body = transformStats(tree.body)(tree.symbol)
          ClassDef.copy(tree)(tree.name, constructor, parents, self, body)
        case tree: Import => Import.copy(tree)(transformTerm(tree.expr)(owner), tree.selectors)
        case tree: Export => tree
    def transformTerm(tree: Term)(owner: Symbol): Term =
      tree match
        case Ident(name) => tree
        case Select(qualifier, name) => Select.copy(tree)(transformTerm(qualifier)(owner), name)
        case This(qual) => tree
        case Super(qual, mix) => Super.copy(tree)(transformTerm(qual)(owner), mix)
        case Apply(fun, args) => Apply.copy(tree)(transformTerm(fun)(owner), transformTerms(args)(owner))
        case TypeApply(fun, args) => TypeApply.copy(tree)(transformTerm(fun)(owner), transformTypeTrees(args)(owner))
        case Literal(const) => tree
        case New(tpt) => New.copy(tree)(transformTypeTree(tpt)(owner))
        case Typed(expr, tpt) => Typed.copy(tree)(transformTerm(expr)(owner), transformTypeTree(tpt)(owner))
        case tree: NamedArg => NamedArg.copy(tree)(tree.name, transformTerm(tree.value)(owner))
        case Assign(lhs, rhs) => Assign.copy(tree)(transformTerm(lhs)(owner), transformTerm(rhs)(owner))
        case Block(stats, expr) => Block.copy(tree)(transformStats(stats)(owner), transformTerm(expr)(owner))
        case If(cond, thenp, elsep) => If.copy(tree)(transformTerm(cond)(owner), transformTerm(thenp)(owner), transformTerm(elsep)(owner))
        case Closure(meth, tpt) => Closure.copy(tree)(transformTerm(meth)(owner), tpt)
        case Match(selector, cases) => Match.copy(tree)(transformTerm(selector)(owner), transformCaseDefs(cases)(owner))
        case Return(expr, from) => Return.copy(tree)(transformTerm(expr)(owner), from)
        case While(cond, body) => While.copy(tree)(transformTerm(cond)(owner), transformTerm(body)(owner))
        case Try(block, cases, finalizer) => Try.copy(tree)(transformTerm(block)(owner), transformCaseDefs(cases)(owner), finalizer.map(x => transformTerm(x)(owner)))
        case Repeated(elems, elemtpt) => Repeated.copy(tree)(transformTerms(elems)(owner), transformTypeTree(elemtpt)(owner))
        case Inlined(call, bindings, expansion) => Inlined.copy(tree)(call, transformSubTrees(bindings)(owner), transformTerm(expansion)(owner))
        case _ => tree
    def transformTypeTree(tree: TypeTree)(owner: Symbol): TypeTree =
      tree match
        case Inferred() => tree
        case tree: TypeIdent => tree
        case tree: TypeSelect => TypeSelect.copy(tree)(tree.qualifier, tree.name)
        case tree: TypeProjection => TypeProjection.copy(tree)(tree.qualifier, tree.name)
        case tree: Annotated => Annotated.copy(tree)(tree.arg, tree.annotation)
        case tree: Singleton => Singleton.copy(tree)(transformTerm(tree.ref)(owner))
        case tree: Refined => Refined.copy(tree)(transformTypeTree(tree.tpt)(owner), transformTrees(tree.refinements)(owner).asInstanceOf[List[Definition]])
        case tree: Applied => Applied.copy(tree)(transformTypeTree(tree.tpt)(owner), transformTrees(tree.args)(owner))
        case tree: MatchTypeTree => tree
        case tree: ByName => ByName.copy(tree)(transformTypeTree(tree.result)(owner))
        case tree: LambdaTypeTree => tree
        case tree: TypeBind => tree
        case tree: TypeBlock => tree
        case _ => tree
    def transformCaseDef(tree: CaseDef)(owner: Symbol): CaseDef =
      CaseDef.copy(tree)(transformTree(tree.pattern)(owner), tree.guard.map(x => transformTerm(x)(owner)), transformTerm(tree.rhs)(owner))
    def transformTypeCaseDef(tree: TypeCaseDef)(owner: Symbol): TypeCaseDef = tree
    def transformStats(trees: List[Statement])(owner: Symbol): List[Statement] = trees.mapConserve(x => transformStatement(x)(owner))
    def transformTrees(trees: List[Tree])(owner: Symbol): List[Tree] = trees.mapConserve(x => transformTree(x)(owner))
    def transformTerms(trees: List[Term])(owner: Symbol): List[Term] = trees.mapConserve(x => transformTerm(x)(owner))
    def transformTypeTrees(trees: List[TypeTree])(owner: Symbol): List[TypeTree] = trees.mapConserve(x => transformTypeTree(x)(owner))
    def transformCaseDefs(trees: List[CaseDef])(owner: Symbol): List[CaseDef] = trees.mapConserve(x => transformCaseDef(x)(owner))
    def transformTypeCaseDefs(trees: List[TypeCaseDef])(owner: Symbol): List[TypeCaseDef] = trees.mapConserve(x => transformTypeCaseDef(x)(owner))
    def transformSubTrees[Tr <: Tree](trees: List[Tr])(owner: Symbol): List[Tr] = transformTrees(trees)(owner).asInstanceOf[List[Tr]]
    def transformSubTree[Tr <: Tree](tree: Tr)(owner: Symbol): Tr = transformTree(tree)(owner).asInstanceOf[Tr]

  object Import:
    @js("$quoted") def copy(original: Tree)(expr: Term, selectors: List[Selector]): Import
    @js("$quoted") def unapply(tree: Import): (Term, List[Selector])
  object ImportMethods:
    extension (self: Import)
      @js("$quoted") def expr: Term
      @js("$quoted") def selectors: List[Selector]
  export ImportMethods.*
  object Export:
    @js("$quoted") def unapply(tree: Export): (Term, List[Selector])
  class Selector extends Tree
  object PackageClauseMethods:
    extension (self: PackageClause)
      @js("$quoted") def pid: Ref
      @js("$quoted") def stats: List[Tree]
  export PackageClauseMethods.*
