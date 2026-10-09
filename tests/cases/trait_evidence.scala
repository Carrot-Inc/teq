trait PolicyType[T]:
  def sqlTypeName: String
  def toStr(t: T): String

object PolicyType:
  given PolicyType[Int] with
    def sqlTypeName: String = "Int"
    def toStr(t: Int): String = t.toString
  given PolicyType[String] with
    def sqlTypeName: String = "String"
    def toStr(t: String): String = "'" + t + "'"
  given [T](using inner: PolicyType[T]): PolicyType[Option[T]] with
    def sqlTypeName: String = s"Option[${inner.sqlTypeName}]"
    def toStr(t: Option[T]): String = t.map(inner.toStr).getOrElse("")

trait PolicyBase[T: PolicyType]:
  val key: String
  val default: T
  val description: String
  lazy val policyType: PolicyType[T] = summon[PolicyType[T]]
  lazy val defaultText: String =
    println(s"computing default of $key")
    policyType.toStr(default)
  def show: String = s"$key [${summon[PolicyType[T]].sqlTypeName}] = $defaultText ($description)"
  def render(value: T): String = policyType.toStr(value)

enum BranchPolicy[T: PolicyType](val key: String, val default: T, val description: String) extends PolicyBase[T]:
  case Phone extends BranchPolicy[String](key = "phone", default = "", description = "Contact phone")
  case MaxHolds extends BranchPolicy[Int](
    key         = "max_holds",
    default     = 25,
    description = "Maximum holds per member",
  )
  case Rounding extends BranchPolicy[Option[Int]]("rounding", None, "Rounding")

  def typeName: String = summon[PolicyType[T]].sqlTypeName

enum LibraryPolicy[T: PolicyType](val key: String, val default: T, val description: String) extends PolicyBase[T]:
  case Name extends LibraryPolicy[String]("name", "Acme", "Library name")
  case Custom[T: PolicyType](name: String, fallback: T) extends LibraryPolicy[T]("custom:" + name, fallback, "Custom")

  def typeName: String = summon[PolicyType[T]].sqlTypeName

final class Loc[T: PolicyType](val key: String, val default: T) extends PolicyBase[T]:
  val description: String = "loc " + defaultText

final class Fixed(val default: Int) extends PolicyBase[Int]:
  val key: String = "fixed"
  val description: String = "always an Int"

object Motd extends PolicyBase[String]:
  val key: String = "motd"
  val default: String = "hello"
  val description: String = "Message of the day"

trait Audited[T] extends PolicyBase[T]:
  def audit: String = "audited " + show

final class AuditedLoc[T: PolicyType](val key: String, val default: T) extends Audited[T]:
  val description: String = "audited"

trait Scaled(using val factor: Int):
  lazy val doubled: Int = factor * 2
  def scale(n: Int): Int = n * factor

trait Labelled(using label: String, sep: Char):
  def tag(s: String): String = label + sep + s

final class Meter(val length: Int)(using Int) extends Scaled:
  def scaled: Int = scale(length)

final class Inch(val length: Int) extends Scaled(using 3), Labelled(using "in", ':')

final class Item(val label: String)(using String, Char) extends Labelled

enum Level extends PolicyBase[Int]:
  case Low, High
  val key: String = "level"
  val default: Int = 1
  val description: String = "A level"

def describe[T](s: PolicyBase[T], value: T): String = s.key + " -> " + s.render(value)

@main def main(): Unit =
  println(BranchPolicy.MaxHolds.show)
  println(BranchPolicy.MaxHolds.defaultText)
  for s <- BranchPolicy.values do println(s.show + " " + s.typeName)
  val custom = LibraryPolicy.Custom("limit", Option("x"))
  println(custom.show + " " + custom.typeName)
  println(LibraryPolicy.Name.show)
  println(describe(BranchPolicy.Rounding, Some(5)))
  val loc = Loc("loc", 7)
  println(loc.description)
  println(loc.show)
  println(Loc[Option[String]]("opt", None).show)
  println(Fixed(3).show)
  println(Motd.show)
  println(describe(Motd, "bye"))
  println(AuditedLoc("a", "b").audit)
  given Int = 10
  val m = Meter(4)
  println(m.scaled)
  println(m.doubled + m.factor)
  val i = Inch(2)
  println(i.scale(5))
  println(i.tag("x"))
  given String = "item"
  given Char = '/'
  val item = Item("own")
  println(item.label + " " + item.tag("y"))
  println(Level.High.show)
  println(Level.values.map(_.defaultText).toList)
  subTraitsAndGivens()
  casesAndCopies()

trait Show[T]:
  def show(t: T): String
object Show:
  given Show[Int] with
    def show(t: Int): String = "i" + t
  given Show[String] with
    def show(t: String): String = "s" + t

trait Base[T: Show]:
  def shown(t: T): String = summon[Show[T]].show(t)

trait Sub[T: Show] extends Base[T]:
  def twice(t: T): String = summon[Show[T]].show(t) + shown(t)

trait Printer[T] extends Base[T]:
  def print(t: T): Unit = println(shown(t))

final class Impl[T: Show]() extends Sub[T]

given intPrinter: Printer[Int] with
  override def print(t: Int): Unit = println("given " + shown(t))

given strPrinter: Printer[String] = new StrPrinter()
final class StrPrinter() extends Printer[String]

def printAll[T](ts: List[T])(using p: Printer[T]): Unit = ts.foreach(p.print)

def subTraitsAndGivens(): Unit =
  println(Impl[Int]().twice(1))
  println(Impl[String]().twice("x"))
  printAll(List(1, 2))
  printAll(List("a"))

trait Rendered[T: Show]:
  def value: T
  lazy val rendered: String = summon[Show[T]].show(value)

enum Cell:
  case Empty
  case Num(value: Int) extends Cell, Rendered[Int]
  case Text(value: String) extends Cell, Rendered[String]

  def describe: String = this match
    case Empty => "empty"
    case r: Rendered[?] => r.rendered

final case class Wrapper[T: Show](value: T, tag: String = "w") extends Rendered[T]

def casesAndCopies(): Unit =
  println(List(Cell.Empty, Cell.Num(3), Cell.Text("t")).map(_.describe))
  val w = Wrapper(5)
  println(w.rendered + " " + w.copy(value = 6).rendered + " " + w.copy(tag = "x"))
