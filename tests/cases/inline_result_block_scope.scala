// A block's inline method that another inline method's check demands before the block reaches it is checked in the
// scope of its own place: the imports the block declares before it are entered ahead, and what the block names from
// its start (a class, a type alias, a method, an object) is there. Where a given stands between the two, the demand
// waits for the block's own check, and the caller that nothing calls builds. scalac prints the lines of the
// .expected file.
object Rates:
  val factor = 3

def importBetween(k: Int): Int =
  inline def widened(x: Int) = scaled(x) + 1
  import Rates.factor
  inline def scaled(x: Int) = x * factor
  widened(k)

def importBetweenUncalled: Int =
  inline def widened(x: Int) = scaled(x) + 1
  import Rates.factor
  inline def scaled(x: Int) = x * factor
  7

def givenBetweenUncalled: Int =
  inline def widened(x: Int) = scaled(x) + 1
  given Int = 3
  inline def scaled(x: Int) = x * summon[Int]
  7

def classBetween(k: Int): Int =
  inline def widened(x: Int) = scaled(x) + 1
  class Held(val factor: Int)
  inline def scaled(x: Int) = x * new Held(3).factor
  widened(k)

def aliasBetween(k: Int): Int =
  inline def widened(x: Int) = scaled(x) + 1
  type Count = Int
  inline def scaled(x: Int) = { val factor: Count = 3; x * factor }
  widened(k)

def methodBetween(k: Int): Int =
  inline def widened(x: Int) = scaled(x) + 1
  def factor: Int = 3
  inline def scaled(x: Int) = x * factor
  widened(k)

def objectBetween(k: Int): Int =
  inline def widened(x: Int) = scaled(x) + 1
  object Held { val factor = 3 }
  inline def scaled(x: Int) = x * Held.factor
  widened(k)

def calleeInOuterBlock(k: Int): Int =
  def nested(j: Int): Int =
    inline def widened(x: Int) = scaled(x) + 1
    widened(j)
  import Rates.factor
  inline def scaled(x: Int) = x * factor
  nested(k)

def calleeInInnerBlock(k: Int): Int =
  val got =
    inline def widened(x: Int) = scaled(x) + 1
    import Rates.factor
    inline def scaled(x: Int) = x * factor
    widened(k)
  got

object Shadow:
  def scaled(x: Int): String = "outer"
  def calleeShadowsOuter(k: Int): Int =
    inline def widened(x: Int) = scaled(x) + 1
    import Rates.factor
    inline def scaled(x: Int) = x * factor
    widened(k)

@main def run(): Unit =
  println(importBetween(5))
  println(importBetweenUncalled)
  println(givenBetweenUncalled)
  println(classBetween(5))
  println(aliasBetween(5))
  println(methodBetween(5))
  println(objectBetween(5))
  println(calleeInOuterBlock(5))
  println(calleeInInnerBlock(5))
  println(Shadow.calleeShadowsOuter(5))
