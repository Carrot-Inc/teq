/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// A type in parentheses wherever the grammar takes one: the parentheses are no node, and the construct that holds
// them spans them, as scalameta 4.17's (a refinement of a parenthesized parent from that parent's own start).
object ParenTypes:
  val marker = 0 // assert: DumpLayout.layout
  class Outer { type Inner = Int }
  type Alias = (Int)
  type Fun = (Int) => (Int)
  type FunInner = ((Int)) => Int
  type Args = List[(Int)]
  type Refined = (AnyRef) { type T }
  type And = (Int) & (Any)
  type Or = (Int) | (String)
  type Tup = ((Int), String)
  type Named = (a: Int, b: String)
  type NamedParen = (a: (Int), b: String)
  type Context = (Int) ?=> (Int)
  type Lambda = [X] =>> (List[X])
  type Poly = [X] => (X) => X
  type ByName = (=> Int) => Int
  type Projected = (Outer)#Inner
  type Infix = (Int) Either (String)
  type Matched[X] = (X) match { case Int => (String) case _ => Int }
  type Wild = List[? <: (AnyRef)]
  val declared: (Int) = 1
  def result: (Int) = 1
  def param(x: (Int)): Int = x
  def repeated(xs: (Int)*): Int = 0
  def bounded[T <: (AnyRef)](t: T): T = t
  type Annotated = (Int) @unchecked
  type Lower[T >: (Nothing)] = T
  type Single = (marker.type)
  type Res = Int => (Int)
  type Pair = Tuple2[(Int), (String)]
  def byName(x: => (Int)): Int = x
  val cast = 1.asInstanceOf[(Int)]
  val ascribedType = 1: (Int)
  def typedPattern(x: Any): Int = x match { case y: (Int) => y case _ => 0 }
  val newTyped = new Outer: (Outer)
  trait Selfish { self: (Outer) => }
  // A parameter's type in parentheses in every clause (the ledger of 2865c7af2's pass, rows 4, 5 and 9): a lambda's,
  // ordinary, of two, a colon argument's, polymorphic, context and under its expected type; a class's, case class's,
  // trait's, enum case's, given's, using's, anonymous using's, implicit's, extension's and secondary constructor's;
  // a dependent function type's named parameter; an ascription's type in an argument.
  val lambdaParam = (x: (Int)) => x
  val lambdaParams = (x: Int, y: (Int)) => x + y
  val colonParam = List(1).map: (x: (Int)) =>
    x
  val polyParam = [X] => (x: (X)) => x
  val contextParam = (x: (Int)) ?=> x
  val expectedContext: Int ?=> Int = (x: (Int)) ?=> x
  class ClassParam(a: (Int))
  class ValParam(val a: (Int))
  case class CaseParam(a: (Int))
  trait TraitParam(a: (Int))
  enum EnumParam:
    case One(a: (Int))
  given givenParam(using x: (Int)): String = ""
  def anonymousUsing(using (Int)): Int = 1
  def usingParam(using x: (Int)): Int = x
  def implicitParam(implicit x: (Int)): Int = x
  extension (x: (Int)) def extensionParam: Int = x
  class SecondaryParam(a: Int):
    def this(s: (String)) = this(1)
  type Dependent = (x: (Int)) => Int
  type DependentContext = (x: (Int)) ?=> Int
  val ascribedArgument = List((1): (Int))
