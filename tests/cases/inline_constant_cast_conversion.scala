// A cast is read by its own type whatever stands under or over it: a conversion over a cast and
// a cast over a conversion are no constant, a block ending in a cast is none, and a conversion
// over a cast to a singleton type is the singleton's constant. As a component of a case class
// that is the selector a cast to a singleton is the constant too. And a cast is read where it is
// the tree's outer layer alone: an operation over a cast is no path for a by-value parameter, a
// cast over a cast is read by the outer one's type, and an ascription over a cast is the
// ascription.
case class Box(n: Int)

object Gate:
  transparent inline def byValue(m: Int): String =
    inline m match
      case 9 => "nine"
      case _ => "other"
  transparent inline def conversionOverCast(inline n: Int): String =
    inline n.asInstanceOf[Int].toInt match
      case 9 => "nine"
      case _ => "other"
  transparent inline def castOverConversion(inline n: Int): String =
    inline n.toInt.asInstanceOf[Int] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def plusOverCast(inline n: Int): String =
    inline +n.asInstanceOf[Int] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def componentOverCast(inline n: Int): String =
    inline (n.asInstanceOf[Int].toInt, 3) match
      case (9, 3) => "nine"
      case _ => "other"
  transparent inline def byValueOverCast(inline n: Int): String = byValue(n.asInstanceOf[Int].toInt)
  transparent inline def byValueOverSingleton(inline n: Int): String = byValue(n.asInstanceOf[9].toInt)
  transparent inline def conversionOverSingleton(inline n: Int): String =
    inline n.asInstanceOf[9].toInt match
      case 9 => "nine"
      case _ => "other"
  transparent inline def blockEndingInCast(inline n: Int): String =
    inline { val u = 0; n.asInstanceOf[Int] } match
      case 9 => "nine"
      case _ => "other"
  transparent inline def boxedSingleton(inline n: Int): String =
    inline Box(n.asInstanceOf[9]) match
      case Box(9) => "nine"
      case _ => "other"
  transparent inline def byValueOverOperation(inline n: Int): String = byValue(n.asInstanceOf[9] + 0)
  transparent inline def castOverSingleton(inline n: Int): String =
    inline n.asInstanceOf[9].asInstanceOf[Int] match
      case 9 => "nine"
      case _ => "other"
  transparent inline def ascribedSingleton(inline n: Int): String =
    inline ((n.asInstanceOf[9]: Int), 3) match
      case (9, 3) => "nine"
      case _ => "other"

@main def run(): Unit =
  println("a conversion over a cast: " + Gate.conversionOverCast((9: Int)))
  println("a cast over a conversion: " + Gate.castOverConversion((9: Int)))
  println("a unary plus over a cast: " + Gate.plusOverCast((9: Int)))
  println("a component, a conversion over a cast: " + Gate.componentOverCast((9: Int)))
  println("by value, a conversion over a cast: " + Gate.byValueOverCast((9: Int)))
  println("by value, a conversion over a cast to the singleton: " + Gate.byValueOverSingleton((9: Int)))
  println("a conversion over a cast to the singleton: " + Gate.conversionOverSingleton((9: Int)))
  println("a block ending in a cast: " + Gate.blockEndingInCast(9))
  println("a case class's component under a cast to the singleton: " + Gate.boxedSingleton((9: Int)))
  println("by value, an operation over a cast to the singleton: " + Gate.byValueOverOperation((8: Int)))
  println("a cast over a cast to the singleton: " + Gate.castOverSingleton(8))
  println("a component, an ascription over a cast to the singleton: " + Gate.ascribedSingleton((8: Int)))
