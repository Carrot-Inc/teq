// Two imports that reach one object, by its name and through a value that is it
// (`val alias: A.type = A`), are one reference, as dotty's `addAltImport` keeps one of two equal
// references (`isSameRef`): its extension is selected, not counted twice into an ambiguity that left
// the selection to the companion's (scalac `A`, `A`). Two objects inheriting one trait's extension stay
// two references (`C`, `D`: an ambiguity, the companion's taken).
class R
object R:
  extension (r: R) def pick: String = "companion pick"
  extension (r: R) def same: String = "companion same"
class Q
object A:
  extension (r: R) def pick: String = "A"
  extension (q: Q) def take: String = "A take"
object Paths:
  val alias: A.type = A
trait Ops:
  def label: String
  extension (r: R) def same: String = label
object C extends Ops:
  def label: String = "C"
object D extends Ops:
  def label: String = "D"
object Main:
  def main(args: Array[String]): Unit =
    locally {
      import A.*
      import Paths.alias.*
      println((new R).pick)
      println((new Q).take)
    }
    locally {
      import C.*
      import D.*
      println((new R).same)
    }
