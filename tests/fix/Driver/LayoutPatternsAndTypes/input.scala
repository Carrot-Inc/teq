/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// Patterns, types and the rest as scalameta 4.17.0 lays them out (scalameta.txt): `a | b | c` right-nested, `rest*` a
// `Pat.Repeated`, `given T` a `Pat.Given`, a term pattern's type variable a `Type.Var` over its name; a match type's
// cases a `Type.CasesBlock`, its type variables `Type.Name`s and `_` a `Type.PatWildcard`; a refinement's `Stat.Block`;
// a template's `derives` types after its body; `inline if` from its `inline`; a named tuple's `Term.Assign`s; `end`
// markers beside what they close; an empty template where the parser stands; a splice's name from its `$`.
trait Animal:
  def name: String
case class Dog(name: String) extends Animal derives CanEqual
object Others:
  val marker = 1 // assert: DumpLayout.layout
  class Outer { class Inner }
  type Elem[X] = X match
    case List[t] => t
    case _ => Nothing
  type R = Animal { def name: String }
  type P = Outer#Inner
  type B = (=> Int) => Int
  opaque type Id = Int
  def alt(x: Any): Int = x match
    case 1 | 2 | 3 => 1
    case xs: List[a] => 2
    case all @ Some(_) => 3
    case Seq(first, rest*) => 4
    case Seq(_*) => 5
    case _ => 0
  def givenPat(x: Option[Int]): Int = x match
    case Some(given Int) => 1
    case _ => 0
  inline def inl(x: Int): Int = inline if x > 0 then 1 else 0
  def named = (a = 1, b = 2)
  def ended(n: Int): Int =
    if n > 0 then
      1
    else
      0
    end if
  def looped(): Unit =
    var i = 0
    while i < 1 do
      i += 1
    end while
  def block =
    val inner = 1
    inner
  end block
  export Other.value
object Other:
  val value = 1
  def quoted(x: scala.quoted.Expr[Int])(using scala.quoted.Quotes): scala.quoted.Expr[Int] = '{ $x + 1 }
end Other
