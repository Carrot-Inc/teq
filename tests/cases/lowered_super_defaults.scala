// `super` calls and default arguments in the interpreter's lowered code, each body called often
// enough to be lowered (src/interp/code.rs): a class's call up its chain, stackable traits'
// `super` calls, `super[T]` naming a trait, and defaults of a method on a receiver, of a
// constructor, of a method called on `super`, of a local def, of a by-name parameter, and a
// default reading an earlier clause's parameter.
trait Base:
  def show(x: Int): String = s"base $x"

trait Loud extends Base:
  override def show(x: Int): String = super.show(x).toUpperCase

trait Tagged extends Base:
  override def show(x: Int): String = "<" + super.show(x) + ">"

class Plain extends Base

class Stacked extends Plain with Loud with Tagged:
  def direct(x: Int): String = super[Loud].show(x) + " / " + super[Tagged].show(x)

class Account(val owner: String, val limit: Int = 100, val note: String = "none"):
  def withdraw(amount: Int, fee: Int = limit / 10): Int = limit - amount - fee
  override def toString: String = s"$owner:$limit:$note"

class Premium(owner: String) extends Account(owner, 500):
  override def withdraw(amount: Int, fee: Int = 0): Int = super.withdraw(amount) + fee

object Main:
  def lazyDefault(n: Int)(x: => Int = { println(s"default for $n"); n * 2 }): Int = x + x

  def run(i: Int): String =
    def local(a: Int, b: Int = i * 3): Int = a + b
    val acc = new Account("a" + i)
    val custom = new Account("c" + i, note = "set")
    val premium = new Premium("p" + i)
    List(
      new Stacked().show(i),
      new Stacked().direct(i),
      acc.toString,
      custom.toString,
      acc.withdraw(i).toString,
      acc.withdraw(i, 1).toString,
      premium.withdraw(i).toString,
      local(i).toString,
      local(i, 0).toString,
      lazyDefault(i)().toString
    ).mkString(" | ")

  def main(args: Array[String]): Unit =
    for i <- 1 to 3 do println(run(i))
