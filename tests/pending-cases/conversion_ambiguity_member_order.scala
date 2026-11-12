// The conversions of a class context are tried in `implicitMembers`'
// order, which `sort` keeps for ties (Implicits.scala:1648-1660, in
// `searchImplicit` for every search), so for `object M extends T2, T1`
// scalac 3.8.4 names c2 before c1. Queued: push 6, item 3 (the
// choice of a conversion through AmbiguousImplicits.qualify).
import scala.language.implicitConversions
class R
class HasPick:
  def pick: String = "h"
trait T1:
  given c1: Conversion[R, HasPick] = _ => new HasPick
trait T2:
  given c2: Conversion[R, HasPick] = _ => new HasPick
object M extends T2, T1:
  def run(): Unit = println((new R).pick)
object Main:
  def main(args: Array[String]): Unit = M.run()
