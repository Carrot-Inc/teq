import scala.quoted.*
trait T:
  def m: Int
object P:
  val x = 1
  val c = true
  def run(): List[String] =
    val r01 = { x match
      case 1 => "one"
      case _ => "other"
    }
    val r02 = { if c then
      "yes"
      else
      "no"
    }
    val r03 = { (y: Int) =>
      y + 1
    }
    val r04 = ( x match
      case 1 => "one"
      case _ => "other"
    )
    val r05 = ( (y: Int) =>
      y + 1
    )
    val r06 = ( new T:
      def m = 1
    )
    val r07 = { x match
                case 1 => "one"
                case _ => "other"
    }
    val r08 = { if c then
                "yes"
                else
                "no"
    }
    val r09 = { (y: Int) =>
                y + 1
    }
    val r10 = ( x match
                case 1 => "one"
                case _ => "other"
    )
    val r11 = ( (y: Int) =>
                y + 1
    )
    val r12 = ( new T:
                def m = 1
    )
    val r13 = { x match
                  case 1 => "one"
                  case _ => "other"
    }
    val r14 = { if c then
                  "yes"
                  else
                  "no"
    }
    val r15 = { (y: Int) =>
                  y + 1
    }
    val r16 = ( x match
                  case 1 => "one"
                  case _ => "other"
    )
    val r17 = ( (y: Int) =>
                  y + 1
    )
    val r18 = ( new T:
                  def m = 1
    )
    List((r01).toString, (r02).toString, (r03(1)).toString, (r04).toString, (r05(1)).toString, (r06.m).toString, (r07).toString, (r08).toString, (r09(1)).toString, (r10).toString, (r11(1)).toString, (r12.m).toString, (r13).toString, (r14).toString, (r15(1)).toString, (r16).toString, (r17(1)).toString, (r18.m).toString)
object Q:
  def q01(using Quotes): Expr[Any] =
    '{ P.x match
      case 1 => "one"
      case _ => "other"
    }
  def q02(using Quotes): Expr[Any] =
    '{ if true then
      "yes"
      else
      "no"
    }
  def q03(using Quotes): Expr[Any] =
    '{ (y: Int) =>
      y + 1
    }
  def q04(using Quotes): Expr[String] =
    '{ val s: String = ${ P.x match
      case 1 => '{ "one" }
      case _ => '{ "other" }
    }
    s }
  def q05(using Quotes): Expr[String] =
    '{ val s: String = ${ if true then
      '{ "yes" }
      else
      '{ "no" }
    }
    s }
  def q06(using Quotes): Expr[Any] =
    '{ P.x match
       case 1 => "one"
       case _ => "other"
    }
  def q07(using Quotes): Expr[Any] =
    '{ if true then
       "yes"
       else
       "no"
    }
  def q08(using Quotes): Expr[Any] =
    '{ (y: Int) =>
       y + 1
    }
  def q09(using Quotes): Expr[String] =
    '{ val s: String = ${ P.x match
                          case 1 => '{ "one" }
                          case _ => '{ "other" }
    }
    s }
  def q10(using Quotes): Expr[String] =
    '{ val s: String = ${ if true then
                          '{ "yes" }
                          else
                          '{ "no" }
    }
    s }
  def q11(using Quotes): Expr[Any] =
    '{ P.x match
         case 1 => "one"
         case _ => "other"
    }
  def q12(using Quotes): Expr[Any] =
    '{ if true then
         "yes"
         else
         "no"
    }
  def q13(using Quotes): Expr[Any] =
    '{ (y: Int) =>
         y + 1
    }
  def q14(using Quotes): Expr[String] =
    '{ val s: String = ${ P.x match
                            case 1 => '{ "one" }
                            case _ => '{ "other" }
    }
    s }
  def q15(using Quotes): Expr[String] =
    '{ val s: String = ${ if true then
                            '{ "yes" }
                            else
                            '{ "no" }
    }
    s }
@main def main(): Unit = println(P.run().mkString(" "))
// A token after an opening brace, parenthesis, quote or splice on its own line, then a match, an if, a lambda or
// (in parentheses) a template, with the lines after it left of that token, at its column or right of it: the
// braces take the width of their first line break's line, the parentheses the width outside them, and scalac
// reads every one of these as the program means. The quotes and splices are typed, not run. scalac prints the
// eighteen values on one line.
