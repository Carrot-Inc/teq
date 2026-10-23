/*
rules = ["file:../../support/ContractProbes.scala"]
*/
// The probes of the passes over the written extents under their rule, the lints and the output scalafix-core 0.14.9's:
// a given of a parenthesized type a definition; a body a comment of its own line ends a block, whatever the comment
// holds, after a leading comment and through adjacent ones, and at the file's end with no line break after it; an
// inner body's, a match's cases', a type match's cases' or a colon argument's comment no block around the construct
// holding it but its region's; cases aligned with their match, cases past their case's own comment and a constructor's
// block of no statements holding their region's comment; a missing else after a comment at the width around at the
// token after it; a parenthesized body and one ending in `;` a block where a comment of its own line follows; a val
// ending past its body's comment, past an end marker, and a method's block from its separator before its statement
// through its comment, replaced whole; a colon argument's block over its region, a braced block its statement; an
// importee written `given` a name; a colon argument's written lambda over the statements after it and over the `;` its
// statement took, replaced whole.
package driver.contractprobes

object GivenParen { trait T; given (T) with {} } // assert: ContractProbes.given

object CommentUrl:
  def f = // assert: ContractProbes.block
    1
    // see https://example.test
  val z = 2

object CommentSlash:
  def f = // assert: ContractProbes.block
    1
    /* text // end */
  val z = 2

object CommentInner:
  def f = // assert: ContractProbes.block
    1
    /* outer /* inner */ end */
  val z = 2

object CommentWhile:
  def f: Unit =
    while false do
      println(1)
      // body
  val z = 2

object IfComment:
  def f: Unit =
    if true then
      println(1)
      // end // assert: ContractProbes.else
  val z = 2

object LeadingComment:
  def f = // assert: ContractProbes.block
    /* lead */ 1
    // tail
  val z = 2

object AdjacentComments:
  def f = // assert: ContractProbes.block
    1
    /* a */ /* b */
  val z = 2

object CasesComment:
  def f =
    1 match
      case _ => 1 // assert: ContractProbes.cases
      // inner
  val z = 2

object CommentLambda:
  val f = (x: Int) =>
    x
    // end
  val z = 2

object ColonInner:
  def f =
    List(1).map:
      x => x
      // inner
  val z = 2

object TypeMatch:
  type T[X] = X match
    case Int => String // assert: ContractProbes.typeCases
    // cases
  val z = 2

object CasesEnd:
  def f = // assert: ContractProbes.block
    1 match
      case _ => 1 // assert: ContractProbes.cases
      // cases
    end match
  val z = 2

object AlignedCases:
  def f =
    1 match
    case _ => 1 // assert: ContractProbes.cases
    // tail
  val z = 2

object NestedComments:
  def f =
    1 match
      case _ => // assert: ContractProbes.cases
        1
        // inner
      // cases
  val z = 2

class CtorComment(x: Int):
  def this() =
    this(1) // assert: ContractProbes.ctor
    // ctor
  val z = 2

object EndMarker:
  val f =
    1 match
      case _ => 1
    end match
    // outer
  val z = 2

object AbsentElse:
  def f = // assert: ContractProbes.block
    if true then
      1
    // tail // assert: ContractProbes.else
  val z = 2

object ParenComment:
  def f = // assert: ContractProbes.block
    (1)
    // tail
  val z = 2

object SemiComment:
  def f = // assert: ContractProbes.block
    1;
    // tail
  val z = 2

object LeadingSeparator:
  def g =
    ; 1
    // tail
  val z = 2

object ColonBraced:
  def take(x: Any): Any = x
  val g = take:
    ; { 1 }
  val z = 2

object ImportGivenName:
  object Names { val `given` = 1 }
  object Use { import Names.{`given`, *}; val n = `given` } // assert: ContractProbes.importee

object ColonLambdaWritten:
  def take(f: Int => Int): Int = f(0)
  val written = take:
    (x: Int) => x
    1

object ColonLambdaSeparator:
  def take(f: Int => Int): Int = f(0)
  val separated = take:
    (x: Int) => x;

object LiteralEof:
  def f = // assert: ContractProbes.block
    1
    // outer