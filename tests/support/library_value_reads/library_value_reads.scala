// jars: scala-library givenfold-lib
// A library's values read through an inline call (scala-library's `summon`, a client `inline def`): each a call of
// the library's getter where scalac's client calls it, its constant a literal where scalac's folds it; and a String
// method of a literal or of a constant (Console's) a call of the platform's method, as scalac's.
package lvr

import givenfold.{C, Outer, Values}

object Use:
  inline def id[A](x: A): A = x
  import givenfold.Givens.given
  import C.given
  import Outer.Inner.given
  def summonInt: Int = summon[Int]
  def summonLong: Long = summon[Long]
  def summonBoolean: Boolean = summon[Boolean]
  def summonString: String = summon[String]
  def summonCompanion: Char = summon[Char]
  def summonNested: Double = summon[Double]
  def idVal: Int = id(Values.x)
  def idLazy: Int = id(Values.y)
  def idFinal: Int = id(Values.k)
  def idStd: Int = id(math.max(3, 7))
  def idStringLiteral: Int = id("ab".length)
  def idStringConstant: String = id(Console.RED.toUpperCase)
