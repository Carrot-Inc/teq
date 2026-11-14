package jfa

trait Measure extends (String => Int) { def apply(s: String): Int = s.length }
trait Chosen { def apply(n: Option[Int]): Int = 9 }
class Length extends Measure
class Scale extends Chosen with Measure
