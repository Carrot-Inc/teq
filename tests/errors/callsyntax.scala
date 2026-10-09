// expect: value zz is not a member of StringContext
// expect: type mismatch: found String, required Int
// expect: too many arguments: expected 2
// expect: too many arguments: expected 1
// expect: type mismatch: found (Int, Int, Int), required (Int, Int)
// expect: type mismatch: found Boolean, required String | (String, Boolean)
// expect: type mismatch: found (Int, Int), required String | (String, Boolean)

extension (sc: StringContext)
  def sum(args: Int*): Int = args.length

final class Acc:
  infix def add(a: Int, b: Int): Acc = this
  infix def one(a: Int): Acc = this
  infix def pair(p: (Int, Int)): Acc = this
  infix def either(u: String | (String, Boolean)): Acc = this

object cls:
  def :=(classes: (String | (String, Boolean))*): String = ""

@main def run(): Unit =
  val n = 1
  println(zz"no such interpolator $n")
  println(sum"${"text"}")
  val acc = Acc()
  acc add (1, 2, 3)
  acc one (1, 2)
  acc pair (1, 2, 3)
  acc either (1, 2)
  println(cls := ("a", true))
