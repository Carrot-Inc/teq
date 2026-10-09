// jars: scala-library cats-kernel cats-core atto-core
//> using dep org.tpolecat::atto-core:0.9.5
// atto 0.9.5 from its jar: the continuation-passing parser combinators compiled from their
// TASTy bodies, a small grammar over `int`, `letter`, `char`, `string`, `many`, `many1`,
// `sepBy`, `opt`, `|`, `~`, `~>`, `<~`, `map`, `flatMap`, `token`, `parens`, `parseOnly`
// with `.either`, `.option`, `.done`, and the error messages of a failed parse.
import atto.*
import atto.Atto.*
import cats.syntax.all.*

sealed trait Expr
case class Num(n: Int) extends Expr
case class Add(l: Expr, r: Expr) extends Expr
case class Mul(l: Expr, r: Expr) extends Expr
case class Pair(key: String, value: Int)

object Main:
  val ident: Parser[String] = many1(letter).map(_.toList.mkString)
  val pair: Parser[Pair] = (token(ident) <~ token(char('='))) ~ token(int) map { case (k, v) => Pair(k, v) }
  val pairs: Parser[List[Pair]] = sepBy(pair, token(char(',')))

  lazy val factor: Parser[Expr] = token(int).map(Num(_)) | parens(expr)
  lazy val term: Parser[Expr] = factor.flatMap(f => many(token(char('*')) ~> factor).map(_.foldLeft(f)(Mul(_, _))))
  lazy val expr: Parser[Expr] = term.flatMap(t => many(token(char('+')) ~> term).map(_.foldLeft(t)(Add(_, _))))

  def eval(e: Expr): Int = e match
    case Num(n) => n
    case Add(l, r) => eval(l) + eval(r)
    case Mul(l, r) => eval(l) * eval(r)

  def main(args: Array[String]): Unit =
    println("-- primitives")
    println(int.parseOnly("42"))
    println(int.parseOnly("-7 rest"))
    println(int.parseOnly("x"))
    println(letter.parseOnly("a1"))
    println(letter.parseOnly("1"))
    println(digit.parseOnly("7"))
    println(char('x').parseOnly("xy"))
    println(char('x').parseOnly("yx"))
    println(string("hello").parseOnly("hello world"))
    println(string("hello").parseOnly("help"))
    println(anyChar.parseOnly(""))
    println(long.parseOnly("12345678901"))
    println(double.parseOnly("3.25"))
    println(stringCI("abc").parseOnly("ABCd"))

    println("-- combinators")
    println(many(letter).parseOnly("abc1"))
    println(many(letter).parseOnly("1"))
    println(many1(letter).parseOnly("1"))
    println(many1(digit).map(_.toList.mkString.toInt).parseOnly("123x"))
    println(sepBy(int, char(',')).parseOnly("1,2,3"))
    println(sepBy(int, char(',')).parseOnly(""))
    println(sepBy1(int, char(',')).parseOnly("4"))
    println(opt(char('-')).parseOnly("-5"))
    println(opt(char('-')).parseOnly("5"))
    println((string("ab") | string("ac")).parseOnly("ac"))
    println((string("ab") | string("ac")).parseOnly("ad"))
    println((letter ~ digit).parseOnly("a1"))
    println((letter ~> digit).parseOnly("a1"))
    println((letter <~ digit).parseOnly("a1"))
    println((letter <~ digit).parseOnly("ab"))
    println(int.map(_ * 2).parseOnly("21"))
    println(int.flatMap(n => manyN(n, letter)).parseOnly("2abc"))
    println(letter.filter(_.isUpper).parseOnly("a"))
    println(takeWhile(_.isDigit).parseOnly("123abc"))
    println(takeWhile1(_.isDigit).parseOnly("abc"))
    println(stringOf(letterOrDigit).parseOnly("ab1 c"))
    println(token(int).parseOnly("5   x"))
    println(parens(int).parseOnly("(8)"))
    println(parens(int).parseOnly("(8"))
    println(int.named("number").parseOnly("x"))
    println(int.namedOpaque("number").parseOnly("x"))
    println((int <~ endOfInput).parseOnly("12"))
    println((int <~ endOfInput).parseOnly("12x"))
    println(count(2, digit).parseOnly("123"))
    println(oneOf("abc").parseOnly("b"))
    println(noneOf("abc").parseOnly("b"))
    println(skipWhitespace.parseOnly("  x"))
    println(choice(string("x"), string("y")).parseOnly("y"))
    println(letter.many.parseOnly("ab"))
    println(int.sepBy(char(';')).parseOnly("1;2"))
    println(pair.parseOnly("a = 1"))
    println(pairs.parseOnly("a = 1, bc = 2,d=3"))
    println(pairs.parseOnly("a = x"))

    println("-- results")
    val r = int.parseOnly("10")
    println(r.either)
    println(r.option)
    println(r.done)
    println(int.parseOnly("x").either)
    println(int.parseOnly("x").option)
    println(int.parse("12").done)
    println(int.parse("1") match
      case ParseResult.Partial(_) => "partial"
      case ParseResult.Done(rest, a) => s"done $rest $a"
      case ParseResult.Fail(rest, stack, message) => s"fail $rest $stack $message")
    println(int.parseOnly("1x") match
      case ParseResult.Done(rest, a) => s"done '$rest' $a"
      case other => other.toString)
    println(string("abc").parseOnly("abd") match
      case ParseResult.Fail(rest, stack, message) => s"fail '$rest' $stack $message"
      case other => other.toString)
    println(expr.parseOnly("1 + 2 * (3 + 4)").map(eval))
    println(expr.parseOnly("2 * 3 * 4 + 1").either.map(eval))
    println(expr.parseOnly("2 * (").either)
    println((int, char(' ') ~> int).mapN(_ + _).parseOnly("3 4"))
    println(int.void.parseOnly("3"))
    println((int >| "n").parseOnly("3"))
    println((int -| (_ + 1)).parseOnly("3"))
    println((letter || int).parseOnly("5"))
    println(ok(1).parseOnly("z"))
    println(err[Int]("boom").parseOnly("z"))
