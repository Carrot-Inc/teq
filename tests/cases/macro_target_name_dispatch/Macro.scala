// A macro's run calls a method through the parent where the child declares another of its name
// and erasure under another `@targetName`: the expansion is the parent's answer, as under scalac.
package tnd

import scala.quoted.*
import scala.annotation.targetName

class Parent:
  @targetName("ints") def f(x: List[Int]): String = "parent-ints"
class Child extends Parent:
  @targetName("strings") def f(x: List[String]): String = "child-strings"

inline def result: String = ${ impl }

def impl(using Quotes): Expr[String] =
  val c = new Child
  val p: Parent = c
  Expr(p.f(List(1)) + "," + c.f(List("s")))
