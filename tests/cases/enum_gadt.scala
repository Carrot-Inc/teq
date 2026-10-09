enum Key[T](val name: String):
  case UserId extends Key[Int]("uid")
  case Nick extends Key[String]("nick")
  case Pair[A, B](a: Key[A], b: Key[B]) extends Key[(A, B)]("pair")

  def default: T = this match
    case UserId => 0
    case Nick => ""
    case Pair(a, b) => (a.default, b.default)

final class Entry[T](val key: Key[T], val value: T):
  def show: String = key match
    case Key.UserId => "int " + (value + 1)
    case Key.Nick => "str " + value.toUpperCase
    case Key.Pair(_, _) => "pair " + value

def bump[T](k: Key[T], v: T): T = k match
  case Key.UserId => v + 1
  case Key.Nick => v.toUpperCase + "!"
  case Key.Pair(a, b) => v

def show[T](k: Key[T], v: T): String = k match
  case Key.UserId => "int " + (v * 2)
  case Key.Nick => "str " + v.length
  case Key.Pair(a, b) => "pair " + v

def onlyInt(k: Key[Int]): String = k match
  case Key.UserId => "uid"

def onlyString(k: Key[String]): Int = k match
  case Key.Nick => 1

@main def main(): Unit =
  println(bump(Key.UserId, 41))
  println(bump(Key.Nick, "hey"))
  println(bump(Key.Pair(Key.UserId, Key.Nick), (1, "x")))
  println(show(Key.UserId, 21))
  println(show(Key.Nick, "four"))
  println(onlyInt(Key.UserId))
  println(onlyString(Key.Nick))
  println(Key.Pair(Key.UserId, Key.Nick).default)
  println(Entry(Key.UserId, 1).show + ", " + Entry(Key.Nick, "a").show)
  gadtDemo()

enum Expr[A]:
  case IntLit(value: Int) extends Expr[Int]
  case BoolLit(value: Boolean) extends Expr[Boolean]
  case Add(left: Expr[Int], right: Expr[Int]) extends Expr[Int]
  case Eq[X](left: Expr[X], right: Expr[X]) extends Expr[Boolean]
  case Cond(test: Expr[Boolean], yes: Expr[A], no: Expr[A])

def eval[T](e: Expr[T]): T = e match
  case Expr.IntLit(v) => v
  case Expr.BoolLit(v) => v
  case Expr.Add(l, r) => eval(l) + eval(r)
  case Expr.Eq(l, r) => eval(l) == eval(r)
  case Expr.Cond(t, y, n) => if eval(t) then eval(y) else eval(n)

def size[T](e: Expr[T]): Int = e match
  case Expr.IntLit(_) | Expr.BoolLit(_) => 1
  case Expr.Add(l, r) => size(l) + size(r) + 1
  case Expr.Eq(l, r) => size(l) + size(r) + 1
  case Expr.Cond(t, y, n) => size(t) + size(y) + size(n) + 1

def intsOnly(e: Expr[Int]): String = e match
  case Expr.IntLit(v) => "lit " + v
  case Expr.Add(_, _) => "add"
  case Expr.Cond(_, _, _) => "cond"

def gadtDemo(): Unit =
  val e = Expr.Cond(Expr.Eq(Expr.Add(Expr.IntLit(1), Expr.IntLit(2)), Expr.IntLit(3)), Expr.IntLit(10), Expr.IntLit(20))
  val n: Int = eval(e)
  println(n)
  val b: Boolean = eval(Expr.Eq(Expr.BoolLit(true), Expr.BoolLit(false)))
  println(b)
  println(size(e))
  println(intsOnly(e))
  println(intsOnly(Expr.IntLit(5)))
