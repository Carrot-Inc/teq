// A concatenation's or an interpolation's operand that is an `if` over a constant condition, whose branch dotty's
// `FirstTransform` takes: the branch's literal is a constant of the result, and an empty one is dropped, as scalac's
// back end has it.
object C:
  final val b = true
  final val c = "y"
  inline val a = "x"
  final val d = 1
  inline val On = true

inline val flag = true

object Main:
  def main(args: Array[String]): Unit =
    val x = args.length
    val s = "y"
    println("a" + x + (if true then "b" else "c"))
    println("x" + (if true then "0" else s))
    println("x" + (if C.b then "0" else C.c))
    println(C.a + (if C.d == 1 then "0" else "n"))
    println((if !false then "b" else "c") + x + (if flag then "" else "c") + "d")
    println("a" + (if false || true then 1 else 2) + x)
    println("" + (if false then "b" else "c"))
    println("a" + x + (if true then (if false then "n" else "b") else "c") + s)
    println(s"<${if (true) "b" else args(0)}>")
    println((if (true) "" else args(0)) + "|" + args.length)
    println("a" + (if (C.On) "b" else args(0)))
