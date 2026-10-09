// An extension of another receiver, tried first and given up for the conversion of a String, makes
// the search type the conversion's body inside the extension's attempt: the std entered on demand
// there keeps its work, the expansions of the inline calls in that body among it, when the attempt
// is retracted (scalac has no such state to keep).
object Syntax:
  extension (n: Int)
    def lengthCompare(s: String): String = s + n
    def sameElements(s: String): String = s + n
    def lastIndexWhere(s: String): String = s + n

import Syntax.*

@main def run(): Unit =
  println("abc".lengthCompare(2))
  println("abc".sameElements("abc"))
  println("abc".lastIndexWhere(_ == 'c'))
  println(2.lengthCompare("x"))
