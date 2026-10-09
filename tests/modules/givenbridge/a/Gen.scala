package gba

// A generic trait's members a superclass implements under another erasure, givens among them: the class mixing
// the trait in bridges each (`Bridges`), `g()Object` to the inherited given `g()I`, whether the superclass is of
// the same module or read from the products (master bridged the defs and vals, not the givens: `AbstractMethodError`).
trait Gen[A]:
  def d: A
  def v: A
  def g: A
  def gp(using s: String): A
  var x: A

class Base:
  def d: Int = 1
  val v: Int = 2
  given g: Int = 3
  given gp(using s: String): Int = s.length
  var x: Int = 10
