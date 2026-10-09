import scala.annotation.{nowarn, unused}

sealed trait Entry:
  def entryName: String

case object First extends Entry:
  def entryName: String = "first"

case class Other(name: String) extends Entry:
  def entryName: String = name

def nameOf[T <: Entry](value: Entry): String = value match
  case Other(name) => s"other $name"
  case value: T @unchecked => value.entryName

def firstString(xs: List[Any]): String = xs.head match
  case s: String @unchecked => s
  case n: Int @unchecked @nowarn => s"int $n"
  case _ => "?"

@nowarn def ignored(@unused x: Int, y: Int @unchecked): Int = y

@nowarn("msg=never used")
def quiet(): Int =
  @unused val spare = 1
  @nowarn val kept = 2
  kept

class Box(@unused val content: Int, @unused hidden: Int):
  @unused private val unusedField = 0
  def get: Int = content

def unpack(pair: (Int, String)): String =
  val (n, s) = (pair: @unchecked)
  s * n

def force(o: Option[Int]): Int = (o: @unchecked) match
  case Some(v) => v

@main def main(): Unit =
  println(nameOf[First.type](First))
  println(nameOf[Entry](Other("x")))
  println(firstString(List("s", 1)))
  println(firstString(List(1, "s")))
  println(firstString(List(1.5)))
  println(ignored(1, 2))
  println(quiet())
  println(Box(3, 4).get)
  println(unpack((2, "ab")))
  println(force(Some(9)))
  val sum = (1 + 2): @nowarn
  println(sum)
