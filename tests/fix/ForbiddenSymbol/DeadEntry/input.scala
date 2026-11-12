/*
rule = ForbiddenSymbol
ForbiddenSymbol.symbols = [
  "scala.collection.IterableOnce.min",
  "scala.collection.IterableOps.init",
  "scala.collection.ArrayOps.init",
  "scala.collection.StringOps.last",
  "scala.collection.NoSuchClass.m",
  "fix.`input$package`.topLevel",
  "fix.`input$package`.nothing",
  "scala.Predef.ArrowAssoc.`->`",
  "scala.math.package.max"
]
*/
package fix

// A top-level definition is a member of its file's module, `input$package`.
def topLevel(x: Int): Int = x + 1 // assert: ForbiddenSymbol

object DeadEntry:
  val list = List(1, 2, 3)
  val array = Array(1, 2, 3)
  def listMin = list.min
  def listInit = list.init // assert: ForbiddenSymbol
  def arrayInit = array.init // assert: ForbiddenSymbol
  def stringLast = "abc".last // assert: ForbiddenSymbol
  def top = topLevel(1) // assert: ForbiddenSymbol
  // A member of a class nested in an object.
  def arrow = 1 -> 2 // assert: ForbiddenSymbol
  // A member of a package's object.
  def greatest = math.max(1, 2) // assert: ForbiddenSymbol
