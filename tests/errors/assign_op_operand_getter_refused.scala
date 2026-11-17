// expect: 13:13: error: illegal access to protected method $lessinit$greater$default$1 in object PD from object A
// expect: 15:18: error: illegal access to protected method $lessinit$greater$default$1 in object PD from object A
// expect: 2 errors found
// An operand of an assignment operator whose record is a later phase's (a default getter the place may not call:
// the reference's typer types the creation and goes on) fails neither typing of the operation: the assignment
// form stands and the getter's refusal alone is told, on a var and on an element.
package lib:
  class PD protected (val a: Int = 1)
package app:
  object A:
    def g(): Int =
      var x = 1
      x += (new lib.PD {}).a
      val arr = new Array[Int](2)
      arr(0) += (new lib.PD {}).a
      x
