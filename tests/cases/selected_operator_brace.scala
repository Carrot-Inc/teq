// A brace block after a symbolic name selected with a dot belongs to that name, as after a name of letters: an
// argument of the method (`xs.++ { .. }`), a refinement of the type (`O.+ { .. }`), the body of an instance; an infix
// operator's brace stays its right operand. scalac applies the selection (Parsers.simpleExprRest).
class L(val items: List[String]):
  def ::(s: String): L = L(s :: items)
  def ++(more: List[String]): L = L(items ++ more)
  override def toString = items.mkString("[", ",", "]")

object O:
  trait + { def f: Int }

def pairs(kv: (String, L)*): String = kv.map((k, v) => k + "=" + v).mkString(" ")

@main def Test =
  val xs = L(List("a"))
  // an argument of a call, the block opening with comment lines
  println(pairs(
    "k" -> xs.++ {
      // the block's own comment
      // and a second one
      List("b")
    }
  ))
  // a definition's right side, a method's body, a statement followed by another
  val ys = xs.++ { List("c") }
  def more: L = xs.++ { List("d") }
  println((ys, more))
  // the block is the selected method's argument: the receiver is the name's left side
  println(xs.:: { "e" })
  println("f" :: xs)
  // an infix operator's brace is its right operand
  println(xs ++ { List("g") })
  // a type named by a selected operator, with a refinement; an instance and a class of it
  val v: O.+ { def f: Int } = new O.+ { def f = 1 }
  class C extends O.+ { def f = 2 }
  println(v.f + new C().f)
