// Case lambdas over several parameters: the cases match the tuple of the parameters, with
// literal, stable-identifier, nested, type-tested and extractor sub-patterns, guards, a binder
// of the whole tuple, a wildcard case and a MatchError when no case takes the arguments.
object Main:
  def split(text: String, separator: Char): List[String] =
    val (parts, current, _) = text.foldLeft((List.empty[String], "", 0)):
      case ((acc, cur, depth), '[') => (acc, cur + '[', depth + 1)
      case ((acc, cur, depth), ']') => (acc, cur + ']', 0.max(depth - 1))
      case ((acc, cur, 0), `separator`) => (cur :: acc, "", 0)
      case ((acc, cur, depth), other) => (acc, cur + other, depth)
    (current :: parts).reverse

  def describe: (Any, Int) => String =
    case (s: String, n) if n > 1 => s * n
    case (s: String, _) => s
    case (Some(x), n) => "some " + x + "/" + n
    case (None, _) => "none"
    case pair @ (_, 0) => "zero " + pair
    case (_, n) => "other " + n

  def count: (Int, Int, Int) => Int =
    case (0, 0, 0) => 0
    case (a, b, c) if a == b => a + c
    case (a, _, c) => a * c

  def strict: (Int, Int) => String =
    case (1, 2) => "one two"
    case (a, b) => s"$a $b"

  def main(args: Array[String]): Unit =
    println(split("md:hover:[a:b]:x", ':'))
    println(split("a/b/[c/d]", '/'))
    println(split("", ':'))
    println(describe("ab", 3))
    println(describe("ab", 1))
    println(describe(Some(4), 2))
    println(describe(None, 5))
    println(describe(3.5, 0))
    println(describe(3.5, 7))
    println(count(0, 0, 0))
    println(count(2, 2, 3))
    println(count(2, 5, 3))
    println(strict(1, 2))
    println(strict(2, 1))
    val log = scala.collection.mutable.ListBuffer.empty[() => Int]
    val fold = List(1, 2, 3).foldLeft(0):
      case (acc, x) =>
        log += (() => acc * 10 + x)
        acc + x
    println(fold + " " + log.map(_()).toList)
    val partial: (Int, String) => String =
      case (1, s) => "one " + s
    println(partial(1, "x"))
    try println(partial(2, "y"))
    catch case e: MatchError => println("no case")
