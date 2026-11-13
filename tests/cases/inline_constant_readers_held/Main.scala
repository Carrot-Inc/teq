// The readers of tests/cases/inline_constant_readers.scala in a body the definition's check holds
// back (a class made inside a quote of the inline method), which is typed again at each call:
// a method a line, each called once.
import scala.quoted.*

object Lib:
  inline def yes: Boolean = true

object Gate:
  transparent inline def inlineIf_bare0(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if b then new Cell(100).v else 200
    }) }
  transparent inline def inlineIf_bare1(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if b then new Cell(100).v else 200
    }) }
  transparent inline def inlineIf_block2(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if { b } then new Cell(100).v else 200
    }) }
  transparent inline def inlineIf_call3: Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if Lib.yes then new Cell(100).v else 200
    }) }
  transparent inline def inlineIf_singleton4(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if b.asInstanceOf[true] then new Cell(100).v else 200
    }) }
  transparent inline def plainIf_bare5(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if b then new Cell(100).v else scala.compiletime.error("the other branch is kept")
    }) }
  transparent inline def plainIf_bare6(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if b then new Cell(100).v else scala.compiletime.error("the other branch is kept")
    }) }
  transparent inline def plainIf_block7(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if { b } then new Cell(100).v else scala.compiletime.error("the other branch is kept")
    }) }
  transparent inline def plainIf_singleton8(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      if b.asInstanceOf[true] then new Cell(100).v else scala.compiletime.error("the other branch is kept")
    }) }
  transparent inline def operand_bare9(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if !b then 200 else new Cell(100).v
    }) }
  transparent inline def operand_bare10(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if !b then 200 else new Cell(100).v
    }) }
  transparent inline def operand_call11: Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if !Lib.yes then 200 else new Cell(100).v
    }) }
  transparent inline def operand_singleton12(inline b: Boolean): Int =
    ${ Quoted.same('{
      class Cell(val v: Int)
      inline if !b.asInstanceOf[true] then 200 else new Cell(100).v
    }) }
  transparent inline def selector_bare13(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline b match
        case true => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def selector_bare14(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline b match
        case true => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def selector_block15(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline { b } match
        case true => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def selector_call16: String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline Lib.yes match
        case true => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def selector_cast17(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline b.asInstanceOf[Boolean] match
        case true => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def selector_singleton18(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline b.asInstanceOf[true] match
        case true => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def guard_bare19(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline 1 match
        case _ if b => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def guard_bare20(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline 1 match
        case _ if b => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def guard_block21(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline 1 match
        case _ if { b } => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def guard_singleton22(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline 1 match
        case _ if b.asInstanceOf[true] => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def component_bare23(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline (b, 3) match
        case (true, 3) => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def component_cast24(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline (b.asInstanceOf[Boolean], 3) match
        case (true, 3) => new Cell("nine").v
        case _ => "other"
    }) }
  transparent inline def component_singleton25(inline b: Boolean): String =
    ${ Quoted.same('{
      class Cell(val v: String)
      inline (b.asInstanceOf[true], 3) match
        case (true, 3) => new Cell("nine").v
        case _ => "other"
    }) }

@main def run(): Unit =
  println("inlineIf, the literal: " + Gate.inlineIf_bare0(true))
  println("inlineIf, the ascribed literal: " + Gate.inlineIf_bare1((true: Boolean)))
  println("inlineIf, in an empty block: " + Gate.inlineIf_block2((true: Boolean)))
  println("inlineIf, a plain inline call written there: " + Gate.inlineIf_call3)
  println("inlineIf, under a cast to the singleton: " + Gate.inlineIf_singleton4((true: Boolean)))
  println("plainIf, the literal: " + Gate.plainIf_bare5(true))
  println("plainIf, the ascribed literal: " + Gate.plainIf_bare6((true: Boolean)))
  println("plainIf, in an empty block: " + Gate.plainIf_block7((true: Boolean)))
  println("plainIf, under a cast to the singleton: " + Gate.plainIf_singleton8((true: Boolean)))
  println("operand, the literal: " + Gate.operand_bare9(true))
  println("operand, the ascribed literal: " + Gate.operand_bare10((true: Boolean)))
  println("operand, a plain inline call written there: " + Gate.operand_call11)
  println("operand, under a cast to the singleton: " + Gate.operand_singleton12((true: Boolean)))
  println("selector, the literal: " + Gate.selector_bare13(true))
  println("selector, the ascribed literal: " + Gate.selector_bare14((true: Boolean)))
  println("selector, in an empty block: " + Gate.selector_block15((true: Boolean)))
  println("selector, a plain inline call written there: " + Gate.selector_call16)
  println("selector, under a cast to Boolean: " + Gate.selector_cast17((true: Boolean)))
  println("selector, under a cast to the singleton: " + Gate.selector_singleton18((true: Boolean)))
  println("guard, the literal: " + Gate.guard_bare19(true))
  println("guard, the ascribed literal: " + Gate.guard_bare20((true: Boolean)))
  println("guard, in an empty block: " + Gate.guard_block21((true: Boolean)))
  println("guard, under a cast to the singleton: " + Gate.guard_singleton22((true: Boolean)))
  println("component, the literal: " + Gate.component_bare23(true))
  println("component, under a cast to Boolean: " + Gate.component_cast24((true: Boolean)))
  println("component, under a cast to the singleton: " + Gate.component_singleton25((true: Boolean)))
