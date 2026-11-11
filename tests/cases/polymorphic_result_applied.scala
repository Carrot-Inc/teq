object M:
  def conv[T]: T => List[T] = t => List(t)

class K:
  def conv[T]: T => List[T] = t => List(t)

object Lib:
  class Getter[S, A](val get: S => A)

  class Maker[From]:
    def apply[To](f: From => To): Getter[From, To] = new Getter[From, To](f)

  class InlineMaker[From]:
    transparent inline def apply[To](inline f: From => To): Any = new Getter[From, To](f)

  object Make:
    def apply[A] = new Maker[A]

  object MakeInline:
    def apply[A] = new InlineMaker[A]

object Reexports:
  export Lib.{Make, MakeInline}

import Reexports.*
import Lib.Getter

case class Pair(a: Int, b: Int)

object Use:
  val k = new K
  val member: List[String | List[Int]] = (List("a") :+ k.conv[Int](1)) :+ "b"
  val module: List[String | List[Int]] = List("a") :+ M.conv[Int](1) :+ "b"
  val inferred: List[String | List[Int]] = List("a") :+ k.conv(2) :+ "b"
  val exported: List[Getter[Pair, Int]] =
    List(new Getter[Pair, Int](_.a)) :+ Make[Pair](_.a) :+ Make[Pair](_.b)
  val inlined: List[Getter[Pair, Int]] =
    List(new Getter[Pair, Int](_.b))
      :+ MakeInline[Pair](_.a)
      :+ MakeInline[Pair](_.b)
  val direct: List[Getter[Pair, Int]] =
    List(new Getter[Pair, Int](_.a)) :+ Lib.Make[Pair](_.b) :+ Lib.Make.apply[Pair](_.a)

@main def run(): Unit =
  println(Use.member)
  println(Use.module)
  println(Use.inferred)
  println(Use.exported.map(_.get(Pair(1, 2))))
  println(Use.inlined.map(_.get(Pair(1, 2))))
  println(Use.direct.map(_.get(Pair(1, 2))))
