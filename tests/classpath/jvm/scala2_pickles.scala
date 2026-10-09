// jars: scala-library scala2-lib
// std: scala-library
// Definitions from the class files of a library compiled by Scala 2.13, which carry scalac 2's
// pickles and no TASTy (kantan.csv's and scala-bcrypt's, in link mode on the JVM): a package
// object's alias with a companion, implicit scope through a singleton type, an inherited
// implicit def with a context bound, a value implicit class with a default, a sealed hierarchy
// matched on, an object's implicit val, constant, var and lazy val. The pickle is rewritten into
// TASTy for the loader; expected output from scalac 3.8.4 with the same jar.
import scala2lib.*
import scala2lib.ops.*

case class Pid(value: Int)

object Pid {
  implicit val pidCell: Cell[Pid] = implicitly[Cell[Int]].contramap(_.value)
}

enum Kind { case A, B }

def shapeName(s: Shape): String = s match {
  case Square(x) => s"sq $x"
  case Dot => "dot"
}

@main def run(): Unit = {
  val kindCell: Cell[Kind] = Cell.from {
    case Kind.A => "a"
    case Kind.B => "b"
  }
  println(kindCell.encode(Kind.B))
  println(Cell[Option[Int]].encode(Some(3)))
  println(Pid.pidCell.encode(Pid(7)))
  println(List(Pid(1), Pid(2)).asLine(',', defaultHeader*))
  println("hey".shout)
  println("ho".shoutTimes())
  println(describe(Square(2.0)) + " " + describe(Dot) + " " + Square(1.0).sides)
  println(Settings.greeting + Settings.Version + { import Settings.defaultSep; implicitly[Char] })
  Settings.counter += 1
  println(Settings.counter)
  println(shapeName(Square(3.0)) + " " + shapeName(Dot))
}
