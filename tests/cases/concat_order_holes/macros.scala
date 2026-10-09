import scala.quoted.*

object Macros:
  inline def plainHole(inline x: String, inline y: String): String = ${ plainHoleImpl('x, 'y) }
  def plainHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ $x + $y }

  inline def ascribedHole(inline x: String, inline y: String): String = ${ ascribedHoleImpl('x, 'y) }
  def ascribedHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ ($x: String) + $y }

  inline def toStringHole(inline x: String, inline y: String): String = ${ toStringHoleImpl('x, 'y) }
  def toStringHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ $x.toString + $y }

  inline def castHole(inline x: String, inline y: String): String = ${ castHoleImpl('x, 'y) }
  def castHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ $x.asInstanceOf[String] + $y }

  inline def uncheckedHole(inline x: String, inline y: String): String = ${ uncheckedHoleImpl('x, 'y) }
  def uncheckedHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ ($x: @unchecked) + $y }

  inline def blockHole(inline x: String, inline y: String): String = ${ blockHoleImpl('x, 'y) }
  def blockHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ ({ println("  stat"); $x }) + $y }

  inline def ascribedBlockHole(inline x: String, inline y: String): String = ${ ascribedBlockHoleImpl('x, 'y) }
  def ascribedBlockHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ (({ println("  stat"); $x }): String) + $y }

  inline def castBlockHole(inline x: String, inline y: String): String = ${ castBlockHoleImpl('x, 'y) }
  def castBlockHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ ({ println("  stat"); $x }).asInstanceOf[String] + $y }

  inline def ifThenHole(inline x: String, inline y: String): String = ${ ifThenHoleImpl('x, 'y) }
  def ifThenHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ (if true then $x else "") + $y }

  inline def ifElseHole(inline x: String, inline y: String): String = ${ ifElseHoleImpl('x, 'y) }
  def ifElseHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ (if false then "" else $x) + $y }

  inline def ifBothHoles(inline x: String, inline z: String, inline y: String): String = ${ ifBothHolesImpl('x, 'z, 'y) }
  def ifBothHolesImpl(x: Expr[String], z: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ (if true then $x else $z) + $y }

  inline def ifCondHole(inline c: Boolean, inline x: String, inline y: String): String = ${ ifCondHoleImpl('c, 'x, 'y) }
  def ifCondHoleImpl(c: Expr[Boolean], x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ (if $c then $x else "") + $y }

  inline def ifAscribedHole(inline x: String, inline y: String): String = ${ ifAscribedHoleImpl('x, 'y) }
  def ifAscribedHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ (if true then ($x: String) else "") + $y }

  inline def ascribedIfHole(inline x: String, inline y: String): String = ${ ascribedIfHoleImpl('x, 'y) }
  def ascribedIfHoleImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] = '{ ((if true then $x else ""): String) + $y }

  inline def reuse(inline c: Boolean, inline x: Any, inline y: String): String = ${ reuseImpl('c, 'x, 'y) }
  def reuseImpl(c: Expr[Boolean], x: Expr[Any], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val first = If(Literal(BooleanConstant(true)), '{ "" + $x }.asTerm, Expr("").asTerm)
    val second = If(c.asTerm, first.thenp, Expr("other").asTerm).asExprOf[String]
    '{ $second + $y }

  inline def reflected(inline x: String, inline y: String): String = ${ reflectedImpl('x, 'y) }
  def reflectedImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val condition = '{ 1 == 1 }.asTerm
    val selected = If(condition, x.asTerm, Expr("").asTerm).asExprOf[String]
    '{ $selected + $y }

  inline def wholeIfTwice(inline x: String, inline y: String): String = ${ wholeIfTwiceImpl('x, 'y) }
  def wholeIfTwiceImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val i = If(Literal(BooleanConstant(true)), x.asTerm, Expr("").asTerm).asExprOf[String]
    '{ $i + $y + "|" + ($i + $y) }

  inline def ifAsBranch(inline c: Boolean, inline x: String, inline y: String): String = ${ ifAsBranchImpl('c, 'x, 'y) }
  def ifAsBranchImpl(c: Expr[Boolean], x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val i = If(Literal(BooleanConstant(true)), x.asTerm, Expr("").asTerm)
    val outer = If(c.asTerm, i, Expr("other").asTerm).asExprOf[String]
    '{ $outer + $y }

  inline def blockOverTyped(inline x: String, inline y: String): String = ${ blockOverTypedImpl('x, 'y) }
  def blockOverTypedImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val typed = Typed(x.asTerm, TypeTree.of[String])
    val block = Block(List('{ println("  stat") }.asTerm), typed).asExprOf[String]
    '{ $block + $y }

  inline def typedOverTyped(inline x: String, inline y: String): String = ${ typedOverTypedImpl('x, 'y) }
  def typedOverTypedImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val typed = Typed(Typed(x.asTerm, TypeTree.of[String]), TypeTree.of[String]).asExprOf[String]
    '{ $typed + $y }

  inline def ifOverTyped(inline x: String, inline y: String): String = ${ ifOverTypedImpl('x, 'y) }
  def ifOverTypedImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val i = If(Literal(BooleanConstant(true)), Typed(x.asTerm, TypeTree.of[String]), Expr("").asTerm).asExprOf[String]
    '{ $i + $y }

  inline def typedOverIf(inline x: String, inline y: String): String = ${ typedOverIfImpl('x, 'y) }
  def typedOverIfImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val i = If(Literal(BooleanConstant(true)), x.asTerm, Expr("").asTerm)
    val typed = Typed(i, TypeTree.of[String]).asExprOf[String]
    '{ $typed + $y }

  inline def blockOverIf(inline x: String, inline y: String): String = ${ blockOverIfImpl('x, 'y) }
  def blockOverIfImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val i = If(Literal(BooleanConstant(true)), x.asTerm, Expr("").asTerm)
    val block = Block(List('{ println("  stat") }.asTerm), i).asExprOf[String]
    '{ $block + $y }

  inline def ifOverBlock(inline x: String, inline y: String): String = ${ ifOverBlockImpl('x, 'y) }
  def ifOverBlockImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val block = Block(List('{ println("  stat") }.asTerm), x.asTerm)
    val i = If(Literal(BooleanConstant(true)), block, Expr("").asTerm).asExprOf[String]
    '{ $i + $y }

  inline def ifOverSplice(inline x: String, inline y: String): String = ${ ifOverSpliceImpl('x, 'y) }
  def ifOverSpliceImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    val i: Expr[String] = '{ if true then $x else "" }
    '{ $i + $y }

  inline def ascribedCondition(inline c: Boolean, inline x: String, inline y: String): String = ${ ascribedConditionImpl('c, 'x, 'y) }
  def ascribedConditionImpl(c: Expr[Boolean], x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    '{ (if ($c: Boolean) then $x else "") + $y }

  inline def negatedCondition(inline c: Boolean, inline x: String, inline y: String): String = ${ negatedConditionImpl('c, 'x, 'y) }
  def negatedConditionImpl(c: Expr[Boolean], x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    '{ (if !$c then "" else $x) + $y }

object M:
  transparent inline def cat(inline x: Any): String = "" + x
  transparent inline def yes: Boolean = true
  transparent inline def no: Boolean = false
  inline def plainYes: Boolean = true

  inline def discarded(inline x: Any, inline y: String): String = ${ discardedImpl('x, 'y) }
  def discardedImpl(x: Expr[Any], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val original = '{ cat($x) }
    val unused = Typed(original.asTerm, TypeTree.of[String])
    '{ $original + $y }

  inline def both(inline x: Any, inline y: String): String = ${ bothImpl('x, 'y) }
  def bothImpl(x: Expr[Any], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val original = '{ cat($x) }
    val wrapped = Typed(original.asTerm, TypeTree.of[String]).asExprOf[String]
    '{ $original + $y + "|" + ($wrapped + $y) }

  inline def choose(inline x: String, inline y: String): String = ${ chooseImpl('x, 'y) }
  def chooseImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    '{ (if yes then $x else "") + $y }

  inline def choosePlain(inline x: String, inline y: String): String = ${ choosePlainImpl('x, 'y) }
  def choosePlainImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    '{ (if plainYes then $x else "") + $y }

  inline def chooseNot(inline x: String, inline y: String): String = ${ chooseNotImpl('x, 'y) }
  def chooseNotImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    '{ (if !no then $x else "") + $y }

  inline def chooseAnd(inline c: Boolean, inline x: String, inline y: String): String = ${ chooseAndImpl('c, 'x, 'y) }
  def chooseAndImpl(c: Expr[Boolean], x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    '{ (if yes && $c then $x else "") + $y }

  inline def reflectedYes(inline x: String, inline y: String): String = ${ reflectedYesImpl('x, 'y) }
  def reflectedYesImpl(x: Expr[String], y: Expr[String])(using Quotes): Expr[String] =
    import quotes.reflect.*
    val i = If('{ yes }.asTerm, x.asTerm, Expr("").asTerm).asExprOf[String]
    '{ $i + $y }
