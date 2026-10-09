// The upstream of the annotations' check (tests/tasty.sh, "The annotations"): a definition of
// each form of source annotation the writer pickles, and of the forms it withholds, which
// scalac, compiling inspect.scala and use.scala against these pickles, has to read as it reads
// its own.
package annot

import scala.annotation.*
import scala.annotation.meta.*

class Mark extends StaticAnnotation
class Typed[T] extends StaticAnnotation
class Two(a: Int)(b: String) extends StaticAnnotation
class Defs(a: Int = 1, b: String = "x") extends StaticAnnotation
class Consts(i: Int, l: Long, d: Double, f: Float, c: Char, z: Boolean, s: String, n: Null, bt: Byte, sh: Short) extends StaticAnnotation
class Arr(xs: Array[Int]) extends StaticAnnotation
class ArrS(xs: Array[String]) extends StaticAnnotation
class Tag(c: Class[?]) extends StaticAnnotation
class Nest(m: Mark) extends StaticAnnotation
class Vararg(xs: Int*) extends StaticAnnotation
class Ref(a: Any) extends StaticAnnotation
@field @setter class FieldSetter extends StaticAnnotation
@getter class Getter extends StaticAnnotation
@param class OnParam extends StaticAnnotation

enum Color:
  case Red, Green

enum Level:
  @deprecated("low", "1.0") case Low
  @Mark case High(n: Int)

object K:
  final val Answer = 42

@deprecated("old class", "1.0") class Old
@implicitNotFound("no Show for ${A}") trait Show[A]

abstract class TestBase:
  @org.junit.Test def inherited(): Unit = ()

class Lib(@Mark val a: Int, @OnParam val b: Int, @FieldSetter var c: Int, @Mark d: Int) extends TestBase:
  @deprecated("use neu", "1.0") def old: Int = 1
  def neu: Int = 2
  @deprecated def bare: Int = 3
  @deprecated(since = "2.0") def since: Int = 4
  @nowarn("cat=deprecation") def quiet: Int = old
  def loud: Int = old
  @targetName("plus") def +(x: Int): Int = x
  @Typed[String] def typed: Int = 5
  @Two(1)("two") def two: Int = 6
  @Defs() def defs0: Int = 7
  @Defs(b = "y") def defs1: Int = 8
  @Defs(b = "y", a = 3) def defs2: Int = 9
  @Consts(-1, 2L, 3.5, 4.5f, 'c', true, "s", null, 1, 2) def consts: Int = 10
  @Arr(Array(1, 2)) def arr: Int = 11
  @ArrS(Array("a", "b")) def arrs: Int = 12
  @Tag(classOf[String]) def tag: Int = 13
  @Nest(new Mark) def nest: Int = 14
  @Vararg(1, 2, 3) def va: Int = 15
  @Vararg() def va0: Int = 16
  @Ref(Color.Red) def en: Int = 17
  @Ref(K.Answer) def constant: Int = 18
  @Mark @Getter def twice: Int = 19
  @throws[Exception]("why") def thr: Int = 20
  @tailrec final def loop(n: Int): Int = if n == 0 then 0 else loop(n - 1)
  @transient lazy val tr: Int = 21
  @volatile var vol: Int = 22
  @FieldSetter var fs: Int = 23
  @Getter val gv: Int = 24
  @varargs def jv(xs: Int*): Int = 25
  def tp[@Mark T](x: T): T = x
  def pa(@Mark x: Int): Int = x
  @deprecated type Alias = Int
  @org.junit.Test def own(): Unit = ()
  @org.junit.Test(timeout = 50L) def timed(): Unit = ()
  @org.junit.Test(expected = classOf[RuntimeException]) def expecting(): Unit = ()

@companionMethod class OnMethod extends StaticAnnotation
@companionClass class OnClass extends StaticAnnotation

object Implicits:
  @OnMethod @OnClass @Getter implicit class Wrap(val i: Int)

class Secondary(i: Int):
  @Mark def this(@Mark s: String) = this(1)

class Ordered(i: Int, j: Int) extends StaticAnnotation

object Reordered:
  def side(n: Int): Int = n
  @Ordered(j = side(3), i = side(4)) def f: Int = 0

object Givens:
  @Mark given alias: Int = 1
  @Mark given showInt: Show[Int] with {}
  @Mark given showList[T]: Show[List[T]] with {}
  given withUsing(using @Mark x: Int, @Getter y: String): Show[Double] with
    def both = (x, y)

class Stable(@scala.annotation.unchecked.uncheckedStable val x: Int)

class Box[@Mark T]
@Mark object Obj
@main def hello(): Unit = println("hello")
