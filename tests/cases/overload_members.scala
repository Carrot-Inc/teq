//> using scala 3.8.4
trait Format[A]:
  def format(a: A): String
given Format[Int] with
  def format(a: Int) = "#" + a
given Format[String] with
  def format(a: String) = "'" + a + "'"

class Ledger(val name: String):
  private var entries = List.empty[String]
  def record(amount: Int): Ledger = record("entry", amount)
  def record(label: String, amount: Int): Ledger =
    entries = entries :+ (label + "=" + amount)
    this
  def record(items: List[Int]): Ledger =
    items.foreach(i => record(i))
    this
  def record[A](item: A)(using f: Format[A]): Ledger =
    entries = entries :+ f.format(item)
    this
  def total(): Int = entries.length
  def total(prefix: String): Int = entries.count(_.startsWith(prefix))
  def dump: String = entries.mkString(", ")

  def find(p: String => Boolean): Option[String] = entries.find(p)
  def find(label: String): Option[String] = find(_.startsWith(label))

def describe(x: Int): String = "int"
def describe(x: String): String = "string"
def describe[A](xs: List[A]): String = "list of " + xs.length
def describe(x: Int, y: Int): String = describe(x) + "," + describe(y)

def fact(n: Int): Int = fact(n, 1)
def fact(n: Int, acc: Int): Int = if n <= 1 then acc else fact(n - 1, acc * n)

@main def run(): Unit =
  val l = Ledger("main")
  l.record(1).record("rent", 2).record(List(3, 4)).record("text")
  println(l.dump)
  println(l.total())
  println(l.total("entry"))
  println(l.find("rent"))
  println(l.find(_.contains("4")))
  println(describe(1))
  println(describe("a"))
  println(describe(List(1, 2)))
  println(describe(1, 2))
  println(fact(5))
  val d: Int => String = describe
  println(d(3))
  println(List(1, 2).map(describe))
  println(List("x").map(describe))
